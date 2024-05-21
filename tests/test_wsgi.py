import os

import httpx
import pytest


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_scope(wsgi_server, threading_mode):
    payload = 'body_payload'
    async with wsgi_server(threading_mode) as port:
        res = httpx.post(
            f'http://localhost:{port}/info?test=true', content=payload, headers=[('test', 'val1'), ('test', 'val2')]
        )

    assert res.status_code == 200
    assert res.headers['content-type'] == 'application/json'

    data = res.json()
    assert data['scheme'] == 'http'
    assert data['method'] == 'POST'
    assert data['path'] == '/info'
    assert data['query_string'] == 'test=true'
    assert data['headers']['HTTP_HOST'] == f'localhost:{port}'
    assert data['content_length'] == str(len(payload))
    assert data['headers']['HTTP_TEST'] == 'val1,val2'


@pytest.mark.asyncio
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_body(wsgi_server, threading_mode):
    async with wsgi_server(threading_mode) as port:
        res = httpx.post(f'http://localhost:{port}/echo', content='test')

    assert res.status_code == 200
    assert res.text == 'test'


@pytest.mark.asyncio
@pytest.mark.skipif(not bool(os.getenv('PGO_RUN')), reason='not PGO build')
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_body_large(wsgi_server, threading_mode):
    data = ''.join([f'{idx}test'.zfill(8) for idx in range(0, 5000)])
    async with wsgi_server(threading_mode) as port:
        res = httpx.post(f'http://localhost:{port}/echo', content=data)

    assert res.status_code == 200
    assert res.text == data


@pytest.mark.asyncio
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_iterbody(wsgi_server, threading_mode):
    async with wsgi_server(threading_mode) as port:
        res = httpx.get(f'http://localhost:{port}/iterbody')

    assert res.status_code == 200
    assert res.text == 'test' * 3


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_app_error(wsgi_server, threading_mode):
    async with wsgi_server(threading_mode) as port:
        res = httpx.get(f'http://localhost:{port}/err_app')

    assert res.status_code == 500
