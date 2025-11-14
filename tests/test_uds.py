import asyncio
import multiprocessing as mp
import os
import stat
import sys
from contextlib import asynccontextmanager
from functools import partial
from pathlib import Path

import httpx
import pytest

from granian import Granian


IS_WIN = sys.platform == 'win32'


def _serve(**kwargs):
    server = Granian(f'tests.apps.{kwargs["interface"]}:app', **kwargs)
    server.serve()


@asynccontextmanager
async def _server(
    interface, runtime_mode, ws=True, tls=False, task_impl='asyncio', static_mount=False, **server_kwargs
):
    certs_path = Path.cwd() / 'tests' / 'fixtures' / 'tls'
    kwargs = {
        'interface': interface,
        'uds': Path('granian.sock'),
        'loop': 'asyncio',
        'blocking_threads': 1,
        'runtime_mode': runtime_mode,
        'task_impl': task_impl,
        'websockets': ws,
        'workers_kill_timeout': 1,
    }
    if tls:
        if tls == 'private':
            kwargs['ssl_cert'] = certs_path / 'pcert.pem'
            kwargs['ssl_key'] = certs_path / 'pkey.pem'
            kwargs['ssl_key_password'] = 'foobar'  # noqa: S105
        else:
            kwargs['ssl_cert'] = certs_path / 'cert.pem'
            kwargs['ssl_key'] = certs_path / 'key.pem'
    if static_mount:
        kwargs['static_path_mount'] = Path.cwd() / 'tests' / 'fixtures'
    kwargs.update(server_kwargs)

    proc = mp.get_context('spawn').Process(target=_serve, kwargs=kwargs)
    proc.start()
    await asyncio.sleep(1.5)

    try:
        yield
    finally:
        proc.terminate()
        proc.join(timeout=2)
        if proc.is_alive():
            proc.kill()


@pytest.fixture(scope='function')
def asgi_server(**extras):
    return partial(_server, 'asgi', **extras)


@pytest.fixture(scope='function')
def rsgi_server(**extras):
    return partial(_server, 'rsgi', **extras)


@pytest.fixture(scope='function')
def wsgi_server(**extras):
    return partial(_server, 'wsgi', **extras)


@pytest.fixture(scope='function')
def server_tls(request):
    return partial(_server, request.param, tls=True)


@pytest.fixture(scope='function')
def http_client():
    transport = httpx.HTTPTransport(uds='granian.sock', verify=False)
    return httpx.Client(transport=transport)


@pytest.mark.asyncio
@pytest.mark.skipif(IS_WIN, reason='no UDS on win')
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_asgi(asgi_server, runtime_mode, http_client):
    async with asgi_server(runtime_mode, ws=False):
        res = http_client.get('http://granian/info?test=true')

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
    assert data['headers']['host'] == 'granian'
    assert 'http.response.pathsend' in data['extensions']
    assert data['state']['global'] == 'test'


@pytest.mark.asyncio
@pytest.mark.skipif(IS_WIN, reason='no UDS on win')
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_rsgi(rsgi_server, runtime_mode, http_client):
    async with rsgi_server(runtime_mode, ws=False):
        res = http_client.get('http://granian/info?test=true')

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
    assert data['headers']['host'] == 'granian'
    assert not data['authority']


@pytest.mark.asyncio
@pytest.mark.skipif(IS_WIN, reason='no UDS on win')
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_wsgi(wsgi_server, runtime_mode, http_client):
    payload = 'body_payload'
    async with wsgi_server(runtime_mode):
        res = http_client.post(
            'http://granian/info?test=true', content=payload, headers=[('test', 'val1'), ('test', 'val2')]
        )

    assert res.status_code == 200
    assert res.headers['content-type'] == 'application/json'

    data = res.json()
    assert data['scheme'] == 'http'
    assert data['method'] == 'POST'
    assert data['path'] == '/info'
    assert data['query_string'] == 'test=true'
    assert data['headers']['HTTP_HOST'] == 'granian'
    assert data['content_length'] == str(len(payload))
    assert data['headers']['HTTP_TEST'] == 'val1,val2'


@pytest.mark.asyncio
@pytest.mark.skipif(IS_WIN, reason='no UDS on win')
@pytest.mark.parametrize('server_tls', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_https(server_tls, runtime_mode, http_client):
    async with server_tls(runtime_mode, ws=False):
        res = http_client.get('https://granian/info?test=true')

    assert res.status_code == 200
    data = res.json()
    assert data['scheme'] == 'https'


@pytest.mark.asyncio
@pytest.mark.skipif(IS_WIN, reason='no UDS on win')
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_uds_default_file_permission(asgi_server, runtime_mode):
    current_umask = os.umask(0)
    os.umask(current_umask)

    async with asgi_server(runtime_mode, ws=False):
        assert stat.S_IMODE(os.stat('granian.sock').st_mode) == 0o777 - current_umask


@pytest.mark.asyncio
@pytest.mark.skipif(IS_WIN, reason='no UDS on win')
@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_uds_configurable_file_permission(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode, ws=False, uds_permissions=0o666):
        assert stat.S_IMODE(os.stat('granian.sock').st_mode) == 0o666
