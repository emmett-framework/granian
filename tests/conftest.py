import asyncio
import multiprocessing as mp
import socket
from contextlib import asynccontextmanager, closing
from functools import partial
from pathlib import Path

import pytest

from granian import Granian


def _serve(app, **kwargs):
    server = Granian(f'tests.apps.{app}:app', **kwargs)
    server.serve()


@asynccontextmanager
async def _server(interface, app, port, threading_mode, tls=False, extra_args=None):
    certs_path = Path.cwd() / 'tests' / 'fixtures' / 'tls'
    kwargs = {
        'interface': interface,
        'port': port,
        'blocking_threads': 1,
        'threading_mode': threading_mode,
    }
    if extra_args:
        kwargs.update(extra_args)
    if tls:
        if tls == 'private':
            kwargs['ssl_cert'] = certs_path / 'pcert.pem'
            kwargs['ssl_key'] = certs_path / 'pkey.pem'
            kwargs['ssl_key_password'] = 'foobar'  # noqa: S105
        else:
            kwargs['ssl_cert'] = certs_path / 'cert.pem'
            kwargs['ssl_key'] = certs_path / 'key.pem'

    succeeded, spawn_failures = False, 0
    while spawn_failures < 3:
        proc = mp.get_context('spawn').Process(target=_serve, args=(app,), kwargs=kwargs)
        proc.start()
        conn_failures = 0
        while conn_failures < 3:
            try:
                await asyncio.sleep(1.5)
                sock = socket.create_connection(('127.0.0.1', port), timeout=1)
                sock.close()
                succeeded = True
                break
            except Exception:
                conn_failures += 1
        if succeeded:
            break

        proc.terminate()
        proc.join()
        spawn_failures += 1

    if not succeeded:
        raise RuntimeError('Cannot bind server')

    try:
        yield port
    finally:
        proc.terminate()
        proc.join()


@pytest.fixture(scope='function')
def server_port():
    with closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as sock:
        sock.bind(('localhost', 0))
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        return sock.getsockname()[1]


@pytest.fixture(scope='function')
def asgi_server(server_port):
    return partial(_server, 'asgi', 'asgi', server_port)


@pytest.fixture(scope='function')
def rsgi_server(server_port):
    return partial(_server, 'rsgi', 'rsgi', server_port)


@pytest.fixture(scope='function')
def wsgi_server(server_port):
    return partial(_server, 'wsgi', 'wsgi', server_port)


@pytest.fixture(scope='function')
def server(server_port, request):
    return partial(_server, request.param, request.param, server_port)


@pytest.fixture(scope='function')
def server_tls(server_port, request):
    return partial(_server, request.param, request.param, server_port, tls=True)


@pytest.fixture(scope='function')
def server_app(server_port):
    return partial(_server, port=server_port)
