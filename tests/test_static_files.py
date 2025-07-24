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
@pytest.mark.parametrize('encoding, encoded_size', [(None, 141), ('gzip', 104), ('br', 55)])
async def test_static_files_precompressed(server_static_files, runtime_mode, encoding, encoded_size):
    async with server_static_files(runtime_mode, ws=False) as port:
        res = httpx.get(
            f'http://localhost:{port}/static/precompressed.txt',
            headers={'accept-encoding': encoding or ''},
        )

    assert res.status_code == 200
    assert res.num_bytes_downloaded == encoded_size
    assert res.headers.get('content-encoding') == encoding
    assert res.headers.get('vary') == ('accept-encoding' if encoding is not None else None)
