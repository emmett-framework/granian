import json
import os

import pytest
import websockets


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_messages(server, threading_mode):
    async with server(threading_mode) as port:
        async with websockets.connect(f'ws://localhost:{port}/ws_echo') as ws:
            await ws.send('foo')
            res_text = await ws.recv()
            await ws.send(b'foo')
            res_bytes = await ws.recv()

    assert res_text == 'foo'
    assert res_bytes == b'foo'


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_reject(server, threading_mode):
    async with server(threading_mode) as port:
        with pytest.raises(websockets.InvalidStatusCode) as exc:
            async with websockets.connect(f'ws://localhost:{port}/ws_reject'):
                pass

    assert exc.value.status_code == 403


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_asgi_scope(asgi_server, threading_mode):
    async with asgi_server(threading_mode) as port:
        async with websockets.connect(f'ws://localhost:{port}/ws_info?test=true') as ws:
            res = await ws.recv()

    data = json.loads(res)
    assert data['asgi'] == {'version': '3.0', 'spec_version': '2.3'}
    assert data['type'] == 'websocket'
    assert data['http_version'] == '1.1'
    assert data['scheme'] == 'ws'
    assert data['path'] == '/ws_info'
    assert data['query_string'] == 'test=true'
    assert data['headers']['host'] == f'localhost:{port}'
    assert not data['subprotocols']


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_rsgi_scope(rsgi_server, threading_mode):
    async with rsgi_server(threading_mode) as port:
        async with websockets.connect(f'ws://localhost:{port}/ws_info?test=true') as ws:
            res = await ws.recv()

    data = json.loads(res)
    assert data['proto'] == 'ws'
    assert data['http_version'] == '1.1'
    assert data['rsgi_version'] == '1.4'
    assert data['scheme'] == 'http'
    assert data['method'] == 'GET'
    assert data['path'] == '/ws_info'
    assert data['query_string'] == 'test=true'
    assert data['headers']['host'] == f'localhost:{port}'
    assert not data['authority']
