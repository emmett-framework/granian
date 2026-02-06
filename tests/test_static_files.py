import httpx
import pytest


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
@pytest.mark.parametrize('file_name', ['media.png', 'こんにちは.png'])
async def test_static_files(server_static_files, runtime_mode, file_name):
    async with server_static_files(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/static/{file_name}')

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
async def test_static_files_dir_no_rewrite(server_static_files, runtime_mode):
    async with server_static_files(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/static/empty')

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
async def test_static_files_rewrite_index(server_static_files, runtime_mode):
    async with server_static_files(runtime_mode, ws=False, static_rewrite=True) as port:
        res = httpx.get(f'http://localhost:{port}/static/file_rewrite')

    assert res.status_code == 200


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_rewrite_notfound(server_static_files, runtime_mode):
    async with server_static_files(runtime_mode, ws=False, static_rewrite=True) as port:
        res = httpx.get(f'http://localhost:{port}/static/empty')

    assert res.status_code == 404


@pytest.mark.asyncio
@pytest.mark.parametrize('server_static_files', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_static_files_multi(server_static_files, runtime_mode):
    async with server_static_files(
        runtime_mode, ws=False, static_mount=[('/static', 'static'), ('/other', 'tls')]
    ) as port:
        res1 = httpx.get(f'http://localhost:{port}/static/media.png')
        res2 = httpx.get(f'http://localhost:{port}/other/cert.pem')

    assert res1.status_code == 200
    assert res2.status_code == 200
    assert res1.headers.get('content-type') == 'image/png'
    assert res2.headers.get('content-type') == 'application/x-x509-ca-cert'
