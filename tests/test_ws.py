import asyncio
import base64
import json
import os
import secrets
import socket
from contextlib import closing

import pytest
import websockets
import websockets.exceptions

from granian import Granian
from granian.server.common import Interfaces
from granian.server.embed import Server as EmbeddedGranian
from tests.apps.asgi import app as asgi_app


def test_websocket_ping_options_validation():
    with pytest.raises(ValueError):
        Granian('tests.apps.asgi:app', websocket_ping_interval=0)
    with pytest.raises(ValueError):
        Granian('tests.apps.asgi:app', websocket_ping_interval=-1)
    with pytest.raises(ValueError):
        Granian('tests.apps.asgi:app', websocket_ping_timeout=0)


async def _open_raw_websocket(port, path='/ws_echo'):
    reader, writer = await asyncio.open_connection('127.0.0.1', port)
    key = base64.b64encode(secrets.token_bytes(16)).decode()
    writer.write(
        (
            f'GET {path} HTTP/1.1\r\n'
            f'Host: localhost:{port}\r\n'
            'Upgrade: websocket\r\n'
            'Connection: Upgrade\r\n'
            f'Sec-WebSocket-Key: {key}\r\n'
            'Sec-WebSocket-Version: 13\r\n\r\n'
        ).encode()
    )
    await writer.drain()
    response = await reader.readuntil(b'\r\n\r\n')
    assert response.startswith(b'HTTP/1.1 101')
    return reader, writer


async def _wait_for_server(port):
    for _ in range(20):
        try:
            reader, writer = await asyncio.open_connection('127.0.0.1', port)
        except OSError:
            await asyncio.sleep(0.05)
            continue
        writer.close()
        await writer.wait_closed()
        return
    raise RuntimeError(f'Server on port {port} did not start')


def _free_port():
    with closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


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
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_asgi_keepalive_disconnects_client_without_pong(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode, ws_ping_interval=0.1, ws_ping_timeout=0.2) as port:
        reader, writer = await _open_raw_websocket(port)
        try:
            # server ping carries an 8 bytes sequence payload
            ping = await asyncio.wait_for(reader.readexactly(10), timeout=2)
            assert ping[:2] == b'\x89\x08'
            # no pong is sent back, so the server should close the connection
            close_header = await asyncio.wait_for(reader.readexactly(2), timeout=2)
            assert close_header[0] == 0x88
            close_payload_length = close_header[1] & 0x7F
            await asyncio.wait_for(reader.readexactly(close_payload_length), timeout=2)
            assert await asyncio.wait_for(reader.read(), timeout=2) == b''
        finally:
            writer.close()
            await writer.wait_closed()


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_asgi_keepalive_keeps_responsive_clients_alive(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode, ws_ping_interval=0.1, ws_ping_timeout=0.3) as port:
        async with (
            websockets.connect(f'ws://localhost:{port}/ws_echo', ping_interval=None) as ws1,
            websockets.connect(f'ws://localhost:{port}/ws_echo', ping_interval=None) as ws2,
        ):
            # outlive several ping intervals; the client lib answers pings automatically
            await asyncio.sleep(0.5)
            await ws1.send('first')
            await ws2.send('second')
            assert await ws1.recv() == 'first'
            assert await ws2.recv() == 'second'


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_asgi_keepalive_processes_pong_while_app_is_busy(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode, ws_ping_interval=0.05, ws_ping_timeout=0.1) as port:
        async with websockets.connect(f'ws://localhost:{port}/ws_slow', ping_interval=None) as ws:
            # the app sleeps way past ping interval and timeout before sending
            assert await ws.recv() == 'ready'


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_asgi_keepalive_timeout_closes_transport_while_app_is_busy(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode, ws_ping_interval=0.05, ws_ping_timeout=0.1) as port:
        reader, writer = await _open_raw_websocket(port, '/ws_slow')
        try:
            ping = await asyncio.wait_for(reader.readexactly(10), timeout=2)
            assert ping[:2] == b'\x89\x08'
            close_header = await asyncio.wait_for(reader.readexactly(2), timeout=0.3)
            assert close_header[0] == 0x88
        finally:
            writer.close()
            await writer.wait_closed()


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_asgi_disconnect_preserves_queued_message_order(asgi_server, runtime_mode, tmp_path):
    target = tmp_path / 'ws_ordered_disconnect'
    async with asgi_server(runtime_mode) as port:
        async with websockets.connect(f'ws://localhost:{port}/ws_slow_close_order') as ws:
            await ws.send(str(target.resolve()))
        for _ in range(20):
            if target.exists():
                break
            await asyncio.sleep(0.05)
    assert target.exists()


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_asgi_keepalive_defers_timeout_during_reader_backpressure(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode, ws_ping_interval=0.1, ws_ping_timeout=0.15) as port:
        async with websockets.connect(f'ws://localhost:{port}/ws_backpressured', ping_interval=None) as ws:
            for _ in range(257):
                await ws.send('message')
            assert await asyncio.wait_for(ws.recv(), timeout=2) == 'ready'


@pytest.mark.asyncio
async def test_asgi_keepalive_configuration_is_per_worker(server_port):
    disabled_port = _free_port()
    enabled = EmbeddedGranian(
        asgi_app,
        port=server_port,
        interface=Interfaces.ASGINL,
        websocket_ping_interval=0.05,
        websocket_ping_timeout=0.1,
    )
    disabled = EmbeddedGranian(asgi_app, port=disabled_port, interface=Interfaces.ASGINL)
    enabled_task = asyncio.create_task(enabled.serve())
    disabled_task = None
    try:
        await _wait_for_server(server_port)
        disabled_task = asyncio.create_task(disabled.serve())
        await _wait_for_server(disabled_port)
        reader, writer = await _open_raw_websocket(server_port, '/ws_slow')
        try:
            ping = await asyncio.wait_for(reader.readexactly(10), timeout=1)
            assert ping[:2] == b'\x89\x08'
        finally:
            writer.close()
            await writer.wait_closed()
    finally:
        enabled.stop()
        disabled.stop()
        tasks = [enabled_task]
        if disabled_task is not None:
            tasks.append(disabled_task)
        await asyncio.gather(*tasks)


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
