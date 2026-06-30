import asyncio
import multiprocessing as mp
import os
import sys
from contextlib import asynccontextmanager
from pathlib import Path

import httpx
import pytest

from granian import Granian


IS_WIN = sys.platform == 'win32'


def _serve(**kwargs):
    server = Granian(f'tests.apps.{kwargs["interface"]}:app', **kwargs)
    server.serve()


@asynccontextmanager
async def _server(interface, runtime_mode, **server_kwargs):
    kwargs = {
        'interface': interface,
        'uds': Path('granian.sock'),
        'address': '127.0.0.1',
        'port': 8001,
        'loop': 'asyncio',
        'blocking_threads': 1,
        'runtime_mode': runtime_mode,
        'websockets': False,
        'workers_kill_timeout': 1,
    }
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


@pytest.mark.asyncio
@pytest.mark.skipif(IS_WIN, reason='no UDS on win')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_dual_socket_asgi(runtime_mode):
    # Test both TCP and UDS connectivity
    async with _server('asgi', runtime_mode):
        # Test UDS
        transport_uds = httpx.AsyncHTTPTransport(uds='granian.sock')
        async with httpx.AsyncClient(transport=transport_uds) as client:
            res = await client.get('http://granian/info')
            assert res.status_code == 200
            data = res.json()
            assert data['scheme'] == 'http'

        # Test TCP
        async with httpx.AsyncClient() as client:
            res = await client.get('http://127.0.0.1:8001/info')
            assert res.status_code == 200
            data = res.json()
            assert data['scheme'] == 'http'
