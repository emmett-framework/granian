import os
import platform

import httpx
import pytest


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
    assert data['rsgi_version'] == '1.5'
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
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_response_file_partial_basic(rsgi_server, runtime_mode):
    """Test basic range request functionality"""
    async with rsgi_server(runtime_mode, ws=False) as port:
        # Test bytes 0-9 (first 10 bytes) - default values
        res = httpx.get(f'http://localhost:{port}/file_partial')

        assert res.status_code == 206
        assert res.headers['content-range'] == 'bytes 0-9/100'
        assert res.headers['content-length'] == '10'
        assert res.text == '0123456789'


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_response_file_partial_middle_range(rsgi_server, runtime_mode):
    """Test middle range requests"""
    async with rsgi_server(runtime_mode, ws=False) as port:
        # Test bytes 10-19 (second 10 bytes)
        res = httpx.get(f'http://localhost:{port}/file_partial?start=10&end=19')

        assert res.status_code == 206
        assert res.headers['content-range'] == 'bytes 10-19/100'
        assert res.headers['content-length'] == '10'
        assert res.text == '0123456789'  # Pattern repeats


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_response_file_partial_single_byte(rsgi_server, runtime_mode):
    """Test single byte range request"""
    async with rsgi_server(runtime_mode, ws=False) as port:
        # Test single byte at position 5
        res = httpx.get(f'http://localhost:{port}/file_partial?start=5&end=5')

        assert res.status_code == 206
        assert res.headers['content-range'] == 'bytes 5-5/100'
        assert res.headers['content-length'] == '1'
        assert res.text == '5'


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_response_file_partial_last_byte(rsgi_server, runtime_mode):
    """Test last byte of file"""
    async with rsgi_server(runtime_mode, ws=False) as port:
        # Test last byte (position 99)
        res = httpx.get(f'http://localhost:{port}/file_partial?start=99&end=99')

        assert res.status_code == 206
        assert res.headers['content-range'] == 'bytes 99-99/100'
        assert res.headers['content-length'] == '1'
        assert res.text == '9'  # Last digit is 9 in pattern


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_response_file_partial_full_file(rsgi_server, runtime_mode):
    """Test requesting entire file via partial request"""
    async with rsgi_server(runtime_mode, ws=False) as port:
        # Request entire 100-byte file
        res = httpx.get(f'http://localhost:{port}/file_partial?start=0&end=99')

        assert res.status_code == 206
        assert res.headers['content-range'] == 'bytes 0-99/100'
        assert res.headers['content-length'] == '100'
        assert res.text == '0123456789' * 10  # Full pattern


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_response_file_partial_start_beyond_file(rsgi_server, runtime_mode):
    """Test error when start position is beyond file size"""
    async with rsgi_server(runtime_mode, ws=False) as port:
        # Start at byte 150 when file is only 100 bytes
        res = httpx.get(f'http://localhost:{port}/file_partial?start=150&end=160')

        assert res.status_code == 416
        assert res.headers.get('content-range') == 'bytes */100'


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_response_file_partial_end_beyond_file(rsgi_server, runtime_mode):
    """Test auto-correction when end position is beyond file size"""
    async with rsgi_server(runtime_mode, ws=False) as port:
        # End at byte 150 when file is only 100 bytes - should clamp to 99
        res = httpx.get(f'http://localhost:{port}/file_partial?start=90&end=150')

        # Should succeed with clamped range
        assert res.status_code == 206
        assert res.headers['content-range'] == 'bytes 90-99/100'
        assert res.headers['content-length'] == '10'
        assert res.text == '0123456789'  # Last 10 bytes


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_response_file_partial_start_after_end(rsgi_server, runtime_mode):
    """Test error when start is greater than end"""
    async with rsgi_server(runtime_mode, ws=False) as port:
        # Start at 60, end at 40 (invalid range)
        res = httpx.get(f'http://localhost:{port}/file_partial?start=60&end=40')

        assert res.status_code == 416
        assert res.headers.get('content-range') == 'bytes */100'


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
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_app_error(rsgi_server, runtime_mode):
    async with rsgi_server(runtime_mode, ws=False) as port:
        res = httpx.get(f'http://localhost:{port}/err_app')

    assert res.status_code == 500
