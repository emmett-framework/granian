import os
import signal
import time

import httpx
import pytest


def _wait_for_new_pid(port: int, old_pids):
    for retry in range(1, 5):
        res = httpx.get(f'http://localhost:{port}/pid')
        assert res.status_code == 200
        new_pid = res.json()['pid']
        if new_pid not in old_pids:
            assert True, 'Worker successfully restarted'
            return new_pid
        print(f'Worker not restarted, sleeping for {retry} seconds.')
        time.sleep(retry)

    return None


@pytest.mark.asyncio
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_app_worker_restart(wsgi_server, threading_mode):
    async with wsgi_server(threading_mode) as (port, pid):
        res = httpx.get(f'http://localhost:{port}/pid')
        assert res.status_code == 200
        worker_pid = res.json()['pid']

        os.kill(pid, signal.SIGHUP)

        assert _wait_for_new_pid(port, [worker_pid]) is not None


@pytest.mark.asyncio
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_app_worker_graceful_restart(wsgi_server, threading_mode):
    workers_graceful_timeout = 2

    async with wsgi_server(threading_mode, extra_args={'workers_graceful_timeout': workers_graceful_timeout}) as (
        port,
        pid,
    ):
        res = httpx.get(f'http://localhost:{port}/pid')
        assert res.status_code == 200
        worker_pid = res.json()['pid']

        # suspend the worker process to simulate that it hangs
        os.kill(worker_pid, signal.SIGSTOP)

        # restart
        os.kill(pid, signal.SIGHUP)
        worker_pid_after_one_restart = _wait_for_new_pid(port, [worker_pid])
        assert worker_pid_after_one_restart is not None

        # wait until the worker_pid is gone
        time.sleep(workers_graceful_timeout + 0.01)

        # suspend the new worker process to simulate that it hangs
        os.kill(worker_pid_after_one_restart, signal.SIGSTOP)

        # restart a 2nd time
        os.kill(pid, signal.SIGHUP)

        assert _wait_for_new_pid(port, [worker_pid, worker_pid_after_one_restart]) is not None
