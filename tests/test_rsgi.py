import os
import platform
from pathlib import Path

import httpx
import pytest


RANGE_FILE_CONTENT = '0123456789'


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_scope(rsgi_server, runtime_mode):
    async with rsgi_server(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/info?test=true')

    assert res.status_code == 200
    assert res.headers['content-type'] == 'application/json'

    data = res.json()
    assert data['proto'] == 'http'
    assert data['http_version'] == '1.1'
    assert data['rsgi_version'] == '1.6'
    assert data['scheme'] == 'http'
    assert data['method'] == 'GET'
    assert data['path'] == '/info'
    assert data['query_string'] == 'test=true'
    assert data['headers']['host'] == f'localhost:{port}'
    assert not data['authority']


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_body(rsgi_server, runtime_mode):
    async with rsgi_server(runtime_mode, ws=False) as port:
        res = httpx.post(f'http://localhost:{port}/echo', content='test')

    assert res.status_code == 200
    assert res.text == 'test'


@pytest.mark.asyncio
@pytest.mark.skipif(not bool(os.getenv('PGO_RUN')), reason='not PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_body_large(rsgi_server, runtime_mode):
    data = ''.join([f'{idx}test'.zfill(8) for idx in range(0, 5000)])
    async with rsgi_server(runtime_mode, ws=False) as port:
        res = httpx.post(f'http://localhost:{port}/echo', content=data)

    assert res.status_code == 200
    assert res.text == data


@pytest.mark.asyncio
@pytest.mark.skipif(platform.python_implementation() == 'PyPy', reason='RSGI stream broken on PyPy')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_body_stream_req(rsgi_server, runtime_mode):
    data = ''.join([f'{idx}test'.zfill(8) for idx in range(0, 5000)])
    async with rsgi_server(runtime_mode, ws=False) as port:
        res = httpx.post(f'http://localhost:{port}/echos', content=data)

    assert res.status_code == 200
    assert res.text == data


@pytest.mark.asyncio
@pytest.mark.skipif(platform.python_implementation() == 'PyPy', reason='RSGI stream broken on PyPy')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_body_stream_res(rsgi_server, runtime_mode):
    async with rsgi_server(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/stream')

    assert res.status_code == 200
    assert res.text == 'test' * 3


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_file(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/file')

    assert res.status_code == 200
    assert res.headers['content-type'] == 'image/png'
    assert res.headers['content-length'] == '95'


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
@pytest.mark.parametrize('file_range', [(0, 10), (3, 8), (5, 6), (9, 10)])
async def test_file_range(rsgi_server, runtime_mode, file_range, tmp_path: Path):
    temp_file = tmp_path / 'temp_file.txt'
    with temp_file.open('w') as f:
        f.write(RANGE_FILE_CONTENT)

    start, end = file_range
    async with rsgi_server(runtime_mode, ws=False) as port:
        res = httpx.get(
            f'http://localhost:{port}/file_range',
            headers=[('file-path', str(temp_file)), ('range', f'bytes={start}-{end - 1}')],
        )
        assert res.status_code == 206
        assert res.headers['content-range'] == f'bytes {start}-{end - 1}/10'
        assert res.headers['content-length'] == f'{end - start}'
        assert res.text == RANGE_FILE_CONTENT[start:end]


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_file_range_exceeding(rsgi_server, runtime_mode, tmp_path: Path):
    temp_file = tmp_path / 'temp_file.txt'
    with temp_file.open('w') as f:
        f.write(RANGE_FILE_CONTENT)

    async with rsgi_server(runtime_mode, ws=False) as port:
        res = httpx.get(
            f'http://localhost:{port}/file_range', headers=[('file-path', str(temp_file)), ('range', 'bytes=0-20')]
        )
        assert res.status_code == 206
        assert res.headers['content-range'] == 'bytes 0-9/10'
        assert res.headers['content-length'] == '10'
        assert res.text == RANGE_FILE_CONTENT


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_file_range_unsatisfiable(rsgi_server, runtime_mode, tmp_path: Path):
    temp_file = tmp_path / 'temp_file.txt'
    with temp_file.open('w') as f:
        f.write(RANGE_FILE_CONTENT)

    async with rsgi_server(runtime_mode, ws=False) as port:
        res = httpx.get(
            f'http://localhost:{port}/file_range', headers=[('file-path', str(temp_file)), ('range', 'bytes=10-20')]
        )
        assert res.status_code == 416
        assert res.headers['content-range'] == 'bytes */10'
        assert res.headers['4xx-reason'] == 'out'

        res = httpx.get(
            f'http://localhost:{port}/file_range', headers=[('file-path', str(temp_file)), ('range', 'bytes=8-4')]
        )
        assert res.status_code == 416
        assert res.headers['content-range'] == 'bytes */10'
        assert res.headers['4xx-reason'] == 'invalid'


@pytest.mark.asyncio
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_app_error(rsgi_server, runtime_mode):
    async with rsgi_server(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/err_app')

    assert res.status_code == 500
