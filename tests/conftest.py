import asyncio
import os
import socket
from contextlib import asynccontextmanager, closing
from functools import partial
from pathlib import Path

import pytest


@asynccontextmanager
async def _server(interface, port, threading_mode, tls=False):
    certs_path = Path.cwd() / 'tests' / 'fixtures' / 'tls'
    tls_opts = (
        (f"--ssl-certificate {certs_path / 'cert.pem'} " f"--ssl-keyfile {certs_path / 'key.pem'} ") if tls else ''
    )
    cmd = ''.join(
        [
            f'granian --interface {interface} --port {port} ',
            f'--threads 1 --threading-mode {threading_mode} ',
            tls_opts,
            '--opt ' if os.getenv('LOOP_OPT') else '',
            f'tests.apps.{interface}:app',
        ]
    )

    succeeded, spawn_failures = False, 0
    while spawn_failures < 3:
        proc = await asyncio.create_subprocess_shell(cmd, env=dict(os.environ))
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
        await proc.wait()
        spawn_failures += 1

    if not succeeded:
        raise RuntimeError('Cannot bind server')

    try:
        yield port
    finally:
        proc.terminate()
        await proc.wait()


@pytest.fixture(scope='function')
def server_port():
    with closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as sock:
        sock.bind(('localhost', 0))
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        return sock.getsockname()[1]


@pytest.fixture(scope='function')
def asgi_server(server_port):
    return partial(_server, 'asgi', server_port)


@pytest.fixture(scope='function')
def rsgi_server(server_port):
    return partial(_server, 'rsgi', server_port)


@pytest.fixture(scope='function')
def wsgi_server(server_port):
    return partial(_server, 'wsgi', server_port)


@pytest.fixture(scope='function')
def server(server_port, request):
    return partial(_server, request.param, server_port)


@pytest.fixture(scope='function')
def server_tls(server_port, request):
    return partial(_server, request.param, server_port, tls=True)
