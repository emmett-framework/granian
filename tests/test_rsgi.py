import os

import httpx
import pytest


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_scope(rsgi_server, threading_mode):
    async with rsgi_server(threading_mode) as port:
        res = httpx.get(f'http://localhost:{port}/info?test=true')

    assert res.status_code == 200
    assert res.headers['content-type'] == 'application/json'

    data = res.json()
    assert data['proto'] == 'http'
    assert data['http_version'] == '1.1'
    assert data['rsgi_version'] == '1.2'
    assert data['scheme'] == 'http'
    assert data['method'] == 'GET'
    assert data['path'] == '/info'
    assert data['query_string'] == 'test=true'
    assert data['headers']['host'] == f'localhost:{port}'


@pytest.mark.asyncio
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_body(rsgi_server, threading_mode):
    async with rsgi_server(threading_mode) as port:
        res = httpx.post(f'http://localhost:{port}/echo', content='test')

    assert res.status_code == 200
    assert res.text == 'test'


@pytest.mark.asyncio
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_serve_partial_file(rsgi_server, threading_mode, tmp_path):
    test_body = 'test123'
    tmp_path.joinpath('test.txt').write_text(test_body)
    async with rsgi_server(threading_mode) as port:
        with httpx.Client() as client:
            headers = {'Range': 'bytes=1-3'}
            res = client.get(f'http://localhost:{port}/file', headers=headers)
            part_of_file = res.content  # This contains the requested part of the file

            assert res.status_code == 216
            assert res.text == part_of_file[0:2]


@pytest.mark.asyncio
@pytest.mark.skipif(not bool(os.getenv('PGO_RUN')), reason='not PGO build')
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_body_large(rsgi_server, threading_mode):
    data = ''.join([f'{idx}test'.zfill(8) for idx in range(0, 5000)])
    async with rsgi_server(threading_mode) as port:
        res = httpx.post(f'http://localhost:{port}/echo', content=data)

    assert res.status_code == 200
    assert res.text == data


@pytest.mark.asyncio
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_body_stream_req(rsgi_server, threading_mode):
    data = ''.join([f'{idx}test'.zfill(8) for idx in range(0, 5000)])
    async with rsgi_server(threading_mode) as port:
        res = httpx.post(f'http://localhost:{port}/echos', content=data)

    assert res.status_code == 200
    assert res.text == data


@pytest.mark.asyncio
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_body_stream_res(rsgi_server, threading_mode):
    async with rsgi_server(threading_mode) as port:
        res = httpx.get(f'http://localhost:{port}/stream')

    assert res.status_code == 200
    assert res.text == 'test' * 3


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_app_error(rsgi_server, threading_mode):
    async with rsgi_server(threading_mode) as port:
        res = httpx.get(f'http://localhost:{port}/err_app')

    assert res.status_code == 500
