import asyncio
import base64
import json
import os

import pytest
import websockets
import websockets.exceptions


async def _raw_ws_connect(port):
    # a bare socket client: it never answers pings on its own, unlike `websockets`
    reader, writer = await asyncio.open_connection('127.0.0.1', port)
    key = base64.b64encode(os.urandom(16)).decode()
    writer.write(
        (
            f'GET /ws_echo HTTP/1.1\r\nHost: localhost:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n'
            f'Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n'
        ).encode()
    )
    await writer.drain()
    await reader.readuntil(b'\r\n\r\n')
    return reader, writer


async def _read_control_frame(reader, timeout):
    # server frames are unmasked and control frames carry at most 125 bytes of payload
    head = await asyncio.wait_for(reader.readexactly(2), timeout)
    return head[0] & 0x0F, await reader.readexactly(head[1] & 0x7F)


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
async def test_ws_ping_timeout(server, runtime_mode):
    async with server(runtime_mode, ws_ping_interval=0.5, ws_ping_timeout=0.5) as port:
        reader, writer = await _raw_ws_connect(port)
        try:
            ping_opcode, _ = await _read_control_frame(reader, 3)
            # the ping is never answered, so the server has to close with 1011
            close_opcode, close_payload = await _read_control_frame(reader, 3)
        finally:
            writer.close()

    assert ping_opcode == 0x9
    assert close_opcode == 0x8
    assert int.from_bytes(close_payload[:2], 'big') == 1011


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_ws_ping_keepalive(server, runtime_mode):
    async with server(runtime_mode, ws_ping_interval=0.5, ws_ping_timeout=0.4) as port:
        # the client answers pings automatically, and sends none of its own
        async with websockets.connect(f'ws://localhost:{port}/ws_echo', ping_interval=None) as ws:
            await asyncio.sleep(2)
            await ws.send('foo')
            res = await ws.recv()

    assert res == 'foo'


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_ws_ping_disabled(server, runtime_mode):
    async with server(runtime_mode, ws_ping_interval=0) as port:
        reader, writer = await _raw_ws_connect(port)
        try:
            with pytest.raises(asyncio.TimeoutError):
                await _read_control_frame(reader, 1.5)
        finally:
            writer.close()
