import os
import platform
import signal
import tempfile
import time
from pathlib import Path

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
@pytest.mark.skipif(platform.system() == 'Windows', reason='SIGHUP not available on Windows')
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_app_worker_restart(server_app, threading_mode):
    with tempfile.TemporaryDirectory() as tmp_dir:
        pid_file_path = Path(tmp_dir, 'server.pid')
        async with server_app(
            interface='wsgi', app='restart', threading_mode=threading_mode, extra_args={'pid_file': pid_file_path}
        ) as port:
            with pid_file_path.open('r') as pid_fd:
                server_pid = int(pid_fd.read().strip())

            res = httpx.get(f'http://localhost:{port}/pid')
            assert res.status_code == 200
            worker_pid = res.json()['pid']

            os.kill(server_pid, signal.SIGHUP)

            assert _wait_for_new_pid(port, [worker_pid]) is not None


@pytest.mark.asyncio
@pytest.mark.skipif(platform.system() == 'Windows', reason='SIGHUP/SIGSTOP not available on Windows')
@pytest.mark.parametrize('threading_mode', ['runtime', 'workers'])
async def test_app_workers_kill_timeout(server_app, threading_mode):
    workers_kill_timeout = 2
    with tempfile.TemporaryDirectory() as tmp_dir:
        pid_file_path = Path(tmp_dir, 'server.pid')
        async with server_app(
            interface='wsgi',
            app='restart',
            threading_mode=threading_mode,
            extra_args={'workers_kill_timeout': workers_kill_timeout, 'pid_file': pid_file_path},
        ) as port:
            with pid_file_path.open('r') as pid_fd:
                server_pid = int(pid_fd.read().strip())

            res = httpx.get(f'http://localhost:{port}/pid')
            assert res.status_code == 200
            worker_pid = res.json()['pid']

            # suspend the worker process to simulate that it hangs
            os.kill(worker_pid, signal.SIGSTOP)

            # restart
            os.kill(server_pid, signal.SIGHUP)
            worker_pid_after_one_restart = _wait_for_new_pid(port, [worker_pid])
            assert worker_pid_after_one_restart is not None

            # wait until the worker_pid is gone
            time.sleep(workers_kill_timeout + 0.01)

            # suspend the new worker process to simulate that it hangs
            os.kill(worker_pid_after_one_restart, signal.SIGSTOP)

            # restart a 2nd time
            os.kill(server_pid, signal.SIGHUP)

            assert _wait_for_new_pid(port, [worker_pid, worker_pid_after_one_restart]) is not None
