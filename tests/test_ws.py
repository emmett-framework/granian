import asyncio
import base64
import json
import os
import socket
import struct

import pytest
import websockets
import websockets.exceptions


def _ws_handshake(sock, port, path):
    key = base64.b64encode(os.urandom(16)).decode()
    request = (
        f'GET {path} HTTP/1.1\r\n'
        f'Host: localhost:{port}\r\n'
        'Upgrade: websocket\r\n'
        'Connection: Upgrade\r\n'
        f'Sec-WebSocket-Key: {key}\r\n'
        'Sec-WebSocket-Version: 13\r\n'
        '\r\n'
    )
    sock.sendall(request.encode())
    response = b''
    while b'\r\n\r\n' not in response:
        chunk = sock.recv(4096)
        if not chunk:
            raise RuntimeError('connection closed during handshake')
        response += chunk
    assert response.startswith(b'HTTP/1.1 101'), response


def _recv_exact(sock, size):
    data = b''
    while len(data) < size:
        chunk = sock.recv(size - len(data))
        if not chunk:
            raise ConnectionError('connection closed')
        data += chunk
    return data


def _read_frame(sock):
    b1, b2 = _recv_exact(sock, 2)
    opcode = b1 & 0x0F
    length = b2 & 0x7F
    if length == 126:
        length = struct.unpack('!H', _recv_exact(sock, 2))[0]
    elif length == 127:
        length = struct.unpack('!Q', _recv_exact(sock, 8))[0]
    payload = _recv_exact(sock, length) if length else b''
    return opcode, payload


def _raw_ws_client(port, path='/ws_echo'):
    sock = socket.create_connection(('localhost', port), timeout=5)
    try:
        _ws_handshake(sock, port, path)
    except Exception:
        sock.close()
        raise
    return sock


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_messages(server, runtime_mode):
    async with server(runtime_mode) as port:
        async with websockets.connect(f'ws://localhost:{port}/ws_echo') as ws:
            await ws.send('foo')
            res_text = await ws.recv()
            await ws.send(b'foo')
            res_bytes = await ws.recv()

    assert res_text == 'foo'
    assert res_bytes == b'foo'


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_reject(server, runtime_mode):
    async with server(runtime_mode) as port:
        with pytest.raises(websockets.exceptions.InvalidStatus) as exc:
            async with websockets.connect(f'ws://localhost:{port}/ws_reject'):
                pass

    assert exc.value.response.status_code == 403


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_asgi_server_close(asgi_server, runtime_mode, tmp_path):
    target = tmp_path / 'ws_result'

    async with asgi_server(runtime_mode) as port:
        async with websockets.connect(f'ws://localhost:{port}/ws_close') as ws:
            await ws.send(str(target.resolve()))
            try:
                await ws.recv()
            except Exception:
                pass

        # reduce flakyness
        await asyncio.sleep(0.1)

    assert target.exists()


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_asgi_reject_explicit(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode) as port:
        with pytest.raises(websockets.exceptions.InvalidStatus) as exc:
            async with websockets.connect(f'ws://localhost:{port}/ws_rejecte'):
                pass

    assert exc.value.response.status_code == 403


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_asgi_reject_custom(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode) as port:
        with pytest.raises(websockets.exceptions.InvalidStatus) as exc:
            async with websockets.connect(f'ws://localhost:{port}/ws_rejectc'):
                pass

    assert exc.value.response.status_code == 403
    assert exc.value.response.body == b'WebSocket connection denied by application'


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_asgi_scope(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode) as port:
        async with websockets.connect(f'ws://localhost:{port}/ws_info?test=true') as ws:
            res = await ws.recv()

        async with websockets.connect(
            f'ws://localhost:{port}/ws_info?test=true', subprotocols=['proto1', 'proto2']
        ) as ws:
            res2 = await ws.recv()

    data = json.loads(res)
    assert data['asgi'] == {'version': '3.0', 'spec_version': '2.3'}
    assert data['type'] == 'websocket'
    assert data['http_version'] == '1.1'
    assert data['scheme'] == 'ws'
    assert data['path'] == '/ws_info'
    assert data['query_string'] == 'test=true'
    assert data['headers']['host'] == f'localhost:{port}'
    assert not data['subprotocols']
    assert 'websocket.http.response' in data['extensions']

    data2 = json.loads(res2)
    assert data2['subprotocols'] == ['proto1', 'proto2']


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_rsgi_scope(rsgi_server, runtime_mode):
    async with rsgi_server(runtime_mode) as port:
        async with websockets.connect(f'ws://localhost:{port}/ws_info?test=true') as ws:
            res = await ws.recv()

    data = json.loads(res)
    assert data['proto'] == 'ws'
    assert data['http_version'] == '1.1'
    assert data['rsgi_version'] == '1.6'
    assert data['scheme'] == 'http'
    assert data['method'] == 'GET'
    assert data['path'] == '/ws_info'
    assert data['query_string'] == 'test=true'
    assert data['headers']['host'] == f'localhost:{port}'
    assert not data['authority']


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_ping_frame_sent(server, runtime_mode):
    async with server(runtime_mode, ws_ping_interval=0.1, ws_ping_timeout=5.0) as port:
        sock = _raw_ws_client(port)
        try:
            opcode, payload = _read_frame(sock)
            assert opcode == 0x9
            assert len(payload) == 4
        finally:
            sock.close()


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_ping_timeout_closes_connection(server, runtime_mode):
    async with server(runtime_mode, ws_ping_interval=0.1, ws_ping_timeout=0.2) as port:
        sock = _raw_ws_client(port)
        try:
            opcode, _ = _read_frame(sock)
            assert opcode == 0x9
            # the client never replies with a pong, so the server must close the connection
            opcode, payload = _read_frame(sock)
            assert opcode == 0x8
            assert struct.unpack('!H', payload[:2])[0] == 1011
        finally:
            sock.close()


@pytest.mark.asyncio
@pytest.mark.parametrize('interval', [None, 0])
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_ping_disabled(server, runtime_mode, interval):
    async with server(runtime_mode, ws_ping_interval=interval, ws_ping_timeout=0.1) as port:
        sock = _raw_ws_client(port)
        try:
            sock.settimeout(0.6)
            with pytest.raises((TimeoutError, socket.timeout)):
                _read_frame(sock)
        finally:
            sock.close()


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_ping_interval_without_timeout(server, runtime_mode):
    async with server(runtime_mode, ws_ping_interval=0.1) as port:
        sock = _raw_ws_client(port)
        try:
            opcode, _ = _read_frame(sock)
            assert opcode == 0x9
            # without a timeout the connection is kept alive and pings keep flowing
            opcode, _ = _read_frame(sock)
            assert opcode == 0x9
        finally:
            sock.close()


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_keepalive_survives_pong(server, runtime_mode):
    async with server(runtime_mode, ws_ping_interval=0.1, ws_ping_timeout=0.3) as port:
        async with websockets.connect(f'ws://localhost:{port}/ws_echo') as ws:
            # the client automatically replies to pings, so the connection must outlive the timeout
            await asyncio.sleep(0.8)
            await ws.send('foo')
            res = await ws.recv()
            assert res == 'foo'
