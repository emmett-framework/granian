import json

import httpx
import pytest
import websockets


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_native_ws_echo(server, runtime_mode):
    async with server(runtime_mode) as port:
        async with websockets.connect(f'ws://localhost:{port}/native_ws') as ws:
            await ws.send('foo')
            res_text = await ws.recv()
            await ws.send(b'foo')
            res_bytes = await ws.recv()

    assert res_text == 'foo'
    assert res_bytes == b'foo'


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_native_ws_broadcast(server, runtime_mode):
    async with server(runtime_mode) as port:
        async with websockets.connect(f'ws://localhost:{port}/native_ws') as ws1:
            async with websockets.connect(f'ws://localhost:{port}/native_ws') as ws2:
                await ws1.send('foo')
                res1 = await ws1.recv()
                res2 = await ws2.recv()

    assert res1 == 'foo'
    assert res2 == 'foo'


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_native_ws_rejects_non_ws_request(server, runtime_mode):
    async with server(runtime_mode) as port:
        async with httpx.AsyncClient() as client:
            res = await client.get(f'http://localhost:{port}/native_ws')

    assert res.status_code == 400


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_native_view(server, runtime_mode):
    async with server(runtime_mode) as port:
        async with httpx.AsyncClient() as client:
            res = await client.get(f'http://localhost:{port}/native_view?foo=bar')

    assert res.status_code == 200
    assert res.headers['content-type'] == 'application/json'

    data = json.loads(res.content)
    assert data['path'] == '/native_view'
    assert data['query'] == 'foo=bar'
    assert data['secure'] is False


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_native_routes_do_not_shadow_app(server, runtime_mode):
    async with server(runtime_mode) as port:
        async with websockets.connect(f'ws://localhost:{port}/ws_echo') as ws:
            await ws.send('foo')
            res = await ws.recv()

        async with httpx.AsyncClient() as client:
            res_http = await client.get(f'http://localhost:{port}/info')

    assert res == 'foo'
    assert res_http.status_code == 200
