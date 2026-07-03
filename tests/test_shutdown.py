import multiprocessing as mp
import os
import signal
import socket
import sys
import time

import httpx
import pytest

from granian import Granian


def _serve(**kwargs):
    server = Granian('tests.apps.asgi:app', **kwargs)
    server.serve()


def _spawn_server(port, runtime_mode):
    kwargs = {
        'interface': 'asgi',
        'port': port,
        'loop': 'asyncio',
        'blocking_threads': 1,
        'runtime_mode': runtime_mode,
        'workers': 1,
    }
    proc = mp.get_context('spawn').Process(target=_serve, kwargs=kwargs)
    proc.start()
    for _ in range(30):
        try:
            sock = socket.create_connection(('127.0.0.1', port), timeout=1)
            sock.close()
            return proc
        except OSError:
            time.sleep(0.5)
    proc.kill()
    raise RuntimeError('Cannot bind server')


@pytest.mark.skipif(bool(os.getenv('PGO_RUN')), reason='PGO build')
@pytest.mark.skipif(sys.platform == 'win32', reason='requires SIGTERM')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
def test_sigterm_shutdown_busy_loop(server_port, runtime_mode):
    proc = _spawn_server(server_port, runtime_mode)
    try:
        res = httpx.get(f'http://localhost:{server_port}/shutdown_race')
        assert res.status_code == 200

        os.kill(proc.pid, signal.SIGTERM)
        proc.join(timeout=10)
        assert not proc.is_alive(), 'worker deadlocked during shutdown'
    finally:
        if proc.is_alive():
            proc.kill()
            proc.join(timeout=5)
