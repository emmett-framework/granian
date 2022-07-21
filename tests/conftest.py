import asyncio
import os
import socket

from contextlib import asynccontextmanager, closing
from functools import partial

import pytest


@asynccontextmanager
async def _server(interface, port, threading_mode):
    proc = await asyncio.create_subprocess_shell(
        f"granian --interface {interface} --port {port} "
        f"--threads 1 --threading-mode {threading_mode} "
        f"tests.apps.{interface}:app",
        env=dict(os.environ)
    )
    await asyncio.sleep(1)
    try:
        yield port
    finally:
        proc.terminate()
        await proc.wait()


@pytest.fixture(scope="function")
def server_port():
    with closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as sock:
        sock.bind(('localhost', 0))
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        return sock.getsockname()[1]


@pytest.fixture(scope="function")
def asgi_server(server_port):
    return partial(_server, "asgi", server_port)


@pytest.fixture(scope="function")
def rsgi_server(server_port):
    return partial(_server, "rsgi", server_port)


@pytest.fixture(scope="function")
def server(server_port, request):
    return partial(_server, request.param, server_port)
