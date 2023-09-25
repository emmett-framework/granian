import json
import pathlib
import ssl

import httpx
import pytest
import websockets


@pytest.mark.asyncio
@pytest.mark.parametrize('server_tls', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_http_scope(server_tls, threading_mode):
    async with server_tls(threading_mode) as port:
        res = httpx.get(f'https://localhost:{port}/info?test=true', verify=False)

    assert res.status_code == 200
    data = res.json()
    assert data['scheme'] == 'https'


@pytest.mark.asyncio
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_asgi_ws_scope(asgi_server, threading_mode):
    ssl_context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    localhost_pem = pathlib.Path.cwd() / 'tests' / 'fixtures' / 'tls' / 'cert.pem'
    ssl_context.load_verify_locations(localhost_pem)

    async with asgi_server(threading_mode, tls=True) as port:
        async with websockets.connect(f'wss://localhost:{port}/ws_info?test=true', ssl=ssl_context) as ws:
            res = await ws.recv()

    data = json.loads(res)
    assert data['scheme'] == 'wss'


@pytest.mark.asyncio
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_rsgi_ws_scope(rsgi_server, threading_mode):
    ssl_context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    localhost_pem = pathlib.Path.cwd() / 'tests' / 'fixtures' / 'tls' / 'cert.pem'
    ssl_context.load_verify_locations(localhost_pem)

    async with rsgi_server(threading_mode, tls=True) as port:
        async with websockets.connect(f'wss://localhost:{port}/ws_info?test=true', ssl=ssl_context) as ws:
            res = await ws.recv()

    data = json.loads(res)
    assert data['scheme'] == 'https'
