import httpx
import pytest


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files(server_static_files, runtime_mode):
    async with server_static_files(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/static/media.png')

    assert res.status_code == 200
    assert res.headers.get('content-type') == 'image/png'
    assert res.headers.get('cache-control')


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_notfound(server_static_files, runtime_mode):
    async with server_static_files(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/static/missing.png')

    assert res.status_code == 404


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_outsidemount(monkeypatch, server_static_files, runtime_mode):
    monkeypatch.setattr(httpx._urlparse, 'normalize_path', lambda v: v)

    async with server_static_files(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/static/../conftest.py')

    assert res.status_code == 404


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_approute(server_static_files, runtime_mode):
    async with server_static_files(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/info')

    assert res.status_code == 200


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_precompressed_identity(server_static_files, runtime_mode):
    async with server_static_files(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/static/precompressed.txt', headers={'accept-encoding': ''})

    assert res.status_code == 200
    assert 'content-encoding' not in res.headers
    assert 'vary' not in res.headers


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_precompressed_gzip(server_static_files, runtime_mode):
    async with server_static_files(runtime_mode, ws=False) as port:
        res = httpx.get(
            f'http://localhost:{port}/static/precompressed.txt',
            headers={'accept-encoding': 'gzip, deflate'},
        )

    assert res.status_code == 200
    assert res.headers['content-encoding'] == 'gzip'
    assert res.headers['vary'] == 'accept-encoding'


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_precompressed_brotli(server_static_files, runtime_mode):
    async with server_static_files(runtime_mode, ws=False) as port:
        res = httpx.get(
            f'http://localhost:{port}/static/precompressed.txt',
            headers={'accept-encoding': 'gzip, deflate, br'},
        )

    assert res.status_code == 200
    assert res.headers['content-encoding'] == 'br'
    assert res.headers['vary'] == 'accept-encoding'
