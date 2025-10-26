import json
import pathlib
import socket
import ssl

import httpx
import pytest
import websockets


@pytest.mark.asyncio
@pytest.mark.parametrize('server_tls', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
@pytest.mark.parametrize(
    'tls_scope', [('tls1.2', ssl.TLSVersion.TLSv1_2, 'TLSv1.2'), ('tls1.3', ssl.TLSVersion.TLSv1_3, 'TLSv1.3')]
)
async def test_tls_protocol(server_tls, runtime_mode, tls_scope):
    tls_input, tls_max_proto, tls_expected = tls_scope

    async with server_tls(runtime_mode, ws=False, tls_proto=tls_input) as port:
        context = ssl.create_default_context()
        context.check_hostname = False
        context.verify_mode = ssl.CERT_NONE
        context.maximum_version = tls_max_proto

        with socket.create_connection(('localhost', port)) as sock:
            with context.wrap_socket(sock, server_hostname='localhost') as ssock:
                tls_version = ssock.version()

        assert tls_version == tls_expected


@pytest.mark.asyncio
@pytest.mark.parametrize('server_tls', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_http_scope(server_tls, runtime_mode):
    async with server_tls(runtime_mode, ws=False) as port:
        res = httpx.get(f'https://localhost:{port}/info?test=true', verify=False)

    assert res.status_code == 200
    data = res.json()
    assert data['scheme'] == 'https'


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_asgi_ws_scope(asgi_server, runtime_mode):
    ssl_context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    localhost_pem = pathlib.Path.cwd() / 'tests' / 'fixtures' / 'tls' / 'cert.pem'
    ssl_context.load_verify_locations(str(localhost_pem))

    async with asgi_server(runtime_mode, tls=True) as port:
        async with websockets.connect(f'wss://localhost:{port}/ws_info?test=true', ssl=ssl_context) as ws:
            res = await ws.recv()

    data = json.loads(res)
    assert data['scheme'] == 'wss'


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_rsgi_ws_scope(rsgi_server, runtime_mode):
    ssl_context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    localhost_pem = pathlib.Path.cwd() / 'tests' / 'fixtures' / 'tls' / 'cert.pem'
    ssl_context.load_verify_locations(str(localhost_pem))

    async with rsgi_server(runtime_mode, tls=True) as port:
        async with websockets.connect(f'wss://localhost:{port}/ws_info?test=true', ssl=ssl_context) as ws:
            res = await ws.recv()

    data = json.loads(res)
    assert data['scheme'] == 'https'


@pytest.mark.asyncio
async def test_tls_encrypted_key(rsgi_server):
    async with rsgi_server('st', ws=False, tls='priv') as port:
        res = httpx.get(f'https://localhost:{port}/info?test=true', verify=False)

    assert res.status_code == 200
    data = res.json()
    assert data['scheme'] == 'https'
