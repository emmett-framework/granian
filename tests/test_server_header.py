import httpx
import pytest


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_server_header_default(server, runtime_mode):
    async with server(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/info')
        res_err = httpx.get(f'http://localhost:{port}/err_app')

    assert res.status_code == 200
    assert res.headers['server'] == 'granian'
    assert res_err.status_code == 500
    assert res_err.headers['server'] == 'granian'


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_server_header_disabled(server, runtime_mode):
    async with server(runtime_mode, ws=False, server_header=False) as port:
        res = httpx.get(f'http://localhost:{port}/info')
        res_err = httpx.get(f'http://localhost:{port}/err_app')

    assert res.status_code == 200
    assert 'server' not in res.headers
    assert res_err.status_code == 500
    assert 'server' not in res_err.headers


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_server_header_disabled_static_files(server_static_files, runtime_mode):
    async with server_static_files(runtime_mode, ws=False, server_header=False) as port:
        res = httpx.get(f'http://localhost:{port}/static/media.png')
        res_missing = httpx.get(f'http://localhost:{port}/static/missing.png')

    assert res.status_code == 200
    assert 'server' not in res.headers
    assert res_missing.status_code == 404
    assert 'server' not in res_missing.headers
