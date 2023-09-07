import httpx
import pytest


@pytest.mark.asyncio
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_scope(wsgi_server, threading_mode):
    payload = 'body_payload'
    async with wsgi_server(threading_mode) as port:
        res = httpx.post(f'http://localhost:{port}/info?test=true', content=payload)

    assert res.status_code == 200
    assert res.headers['content-type'] == 'application/json'

    data = res.json()
    assert data['scheme'] == 'http'
    assert data['method'] == 'POST'
    assert data['path'] == '/info'
    assert data['query_string'] == 'test=true'
    assert data['headers']['HTTP_HOST'] == f'localhost:{port}'
    assert data['content_length'] == str(len(payload))


@pytest.mark.asyncio
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_body(wsgi_server, threading_mode):
    async with wsgi_server(threading_mode) as port:
        res = httpx.post(f'http://localhost:{port}/echo', content='test')

    assert res.status_code == 200
    assert res.text == 'test'


@pytest.mark.asyncio
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_iterbody(wsgi_server, threading_mode):
    async with wsgi_server(threading_mode) as port:
        res = httpx.get(f'http://localhost:{port}/iterbody')

    assert res.status_code == 200
    assert res.text == 'test' * 3


@pytest.mark.asyncio
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_app_error(wsgi_server, threading_mode):
    async with wsgi_server(threading_mode) as port:
        res = httpx.get(f'http://localhost:{port}/err_app')

    assert res.status_code == 500
