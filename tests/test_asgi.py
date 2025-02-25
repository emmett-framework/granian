import os

import httpx
import pytest


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_scope(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode) as port:
        res = httpx.get(f'http://localhost:{port}/info?test=true')

    assert res.status_code == 200
    assert res.headers['content-type'] == 'application/json'

    data = res.json()
    assert data['asgi'] == {'version': '3.0', 'spec_version': '2.3'}
    assert data['type'] == 'http'
    assert data['http_version'] == '1.1'
    assert data['scheme'] == 'http'
    assert data['method'] == 'GET'
    assert data['path'] == '/info'
    assert data['query_string'] == 'test=true'
    assert data['headers']['host'] == f'localhost:{port}'
    assert 'http.response.pathsend' in data['extensions']
    assert data['state']['global'] == 'test'


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_scope_path_utf8_lossy(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode) as port:
        res = httpx.get(f'http://localhost:{port}/%c0')

    assert res.status_code == 200
    data = res.json()
    assert data['path'] == '/\ufffd'


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_body(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode) as port:
        res = httpx.post(f'http://localhost:{port}/echo', content='test')

    assert res.status_code == 200
    assert res.text == 'test'


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_body_large(asgi_server, runtime_mode):
    data = ''.join([f'{idx}test'.zfill(8) for idx in range(0, 5000)])
    async with asgi_server(runtime_mode) as port:
        res = httpx.post(f'http://localhost:{port}/echo', content=data)

    assert res.status_code == 200
    assert res.text == data


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_app_error(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode) as port:
        res = httpx.get(f'http://localhost:{port}/err_app')

    assert res.status_code == 500


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_protocol_error(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode) as port:
        res = httpx.get(f'http://localhost:{port}/err_proto')

    assert res.status_code == 500


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_file(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode) as port:
        res = httpx.get(f'http://localhost:{port}/file')

    assert res.status_code == 200
    assert res.headers['content-type'] == 'image/png'
    assert res.headers['content-length'] == '95'


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_sniffio(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode, task_impl='rust') as port:
        res = httpx.get(f'http://localhost:{port}/sniffio')

    assert res.status_code == 200
    assert res.text == 'asyncio'


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_timeout(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode, task_impl='rust') as port:
        res = httpx.get(f'http://localhost:{port}/timeout_n')

    assert res.status_code == 200
    assert res.text == 'ok'

    async with asgi_server(runtime_mode, task_impl='rust') as port:
        res = httpx.get(f'http://localhost:{port}/timeout_w')

    assert res.status_code == 200
    assert res.text == 'timeout'
