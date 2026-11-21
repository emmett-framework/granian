import asyncio
import multiprocessing as mp
import os
import re
import signal
import socket
import subprocess
import sys
import time
from contextlib import asynccontextmanager, closing
from unittest import mock

import httpx
import pytest
from conftest import _serve

from granian import Granian
from granian._granian import BUILD_GIL


pytestmark = [
    pytest.mark.asyncio,
    pytest.mark.skipif(sys.platform == 'win32', reason='metrics not available on Windows'),
]

WORKER_METRICS = [
    ('worker_lifetime', 'counter'),
    ('connections_active', 'gauge'),
    ('connections_handled', 'counter'),
    ('connections_err', 'counter'),
    ('requests_handled', 'counter'),
    ('static_requests_handled', 'counter'),
    ('static_requests_err', 'counter'),
    ('blocking_threads', 'gauge'),
    ('blocking_queue', 'gauge'),
    ('blocking_idle_cumulative', 'counter'),
    ('blocking_busy_cumulative', 'counter'),
    ('py_wait_cumulative', 'counter'),
]
MAIN_METRICS = [
    'workers_spawns',
    'workers_respawns_for_err',
    'workers_respawns_for_lifetime',
    'workers_respawns_for_rss',
]
SAMPLE_RE = re.compile(r'^granian_(\w+?)(?:\{worker="(\d+)"\})? (-?\d+)$')


def _free_port():
    with closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as sock:
        sock.bind(('localhost', 0))
        return sock.getsockname()[1]


@asynccontextmanager
async def _metrics_server(interface, port, workers=1, **extra):
    metrics_port = _free_port()
    kwargs = {
        'interface': interface,
        'port': port,
        'loop': 'asyncio',
        'blocking_threads': 1,
        'runtime_mode': 'st',
        'workers': workers,
        'metrics_enabled': True,
        'metrics_port': metrics_port,
        'metrics_scrape_interval': 1,
        **extra,
    }
    proc = mp.get_context('spawn').Process(target=_serve, kwargs=kwargs)
    proc.start()
    try:
        # wait for workers to report at least once
        await _wait_for(lambda: len(_workers_in(_scrape(metrics_port)[1], 'requests_handled')) == workers, timeout=15)
        yield proc, port, metrics_port
    finally:
        proc.terminate()
        proc.join(timeout=5)
        if proc.is_alive():
            proc.kill()


def _scrape(metrics_port):
    """Return (response, {(name, worker): value}) validating the exposition format."""
    res = httpx.get(f'http://127.0.0.1:{metrics_port}/', timeout=2)
    samples, declared = {}, set()
    for line in res.text.splitlines():
        if line.startswith('# TYPE '):
            _, _, name, kind = line.split(' ')
            assert kind in ('counter', 'gauge'), line
            declared.add(name)
            continue
        match = SAMPLE_RE.match(line)
        assert match, f'malformed sample line: {line!r}'
        name, worker, value = match.groups()
        assert f'granian_{name}' in declared, f'sample before TYPE: {line!r}'
        samples[(name, worker)] = int(value)
    return res, samples


def _try_scrape(metrics_port):
    try:
        return _scrape(metrics_port)[1]
    except httpx.TransportError:
        return {}


def _workers_in(samples, name):
    return {worker: value for (metric, worker), value in samples.items() if metric == name}


async def _wait_for(predicate, timeout=10, interval=0.2):
    deadline = time.monotonic() + timeout
    while True:
        try:
            if predicate():
                return
        except httpx.TransportError:
            pass
        if time.monotonic() > deadline:
            raise AssertionError('condition not met before timeout')
        await asyncio.sleep(interval)


def _worker_pids(proc):
    out = subprocess.run(['pgrep', '-P', str(proc.pid), '-f', 'spawn_main'], capture_output=True, text=True)  # noqa: S603, S607
    return [int(pid) for pid in out.stdout.split()]


@pytest.mark.parametrize('interface', ['asgi', 'rsgi', 'wsgi'])
async def test_exposition_format(server_port, interface):
    async with _metrics_server(interface, server_port, workers=2) as (_, _, metrics_port):
        res, samples = _scrape(metrics_port)

    assert res.status_code == 200
    assert res.headers['content-type'] == 'text/plain; version=0.0.4; charset=utf-8'
    assert res.text.endswith('\n')
    for name in MAIN_METRICS:
        assert (name, None) in samples
    for name, _ in WORKER_METRICS:
        assert set(_workers_in(samples, name)) == {'1', '2'}, name
    assert samples[('workers_spawns', None)] == 2


@pytest.mark.parametrize('interface', ['asgi', 'rsgi', 'wsgi'])
async def test_requests_counted(server_port, interface):
    requests = 25
    async with _metrics_server(interface, server_port) as (_, port, metrics_port):
        with httpx.Client() as client:
            for _ in range(requests):
                assert client.get(f'http://localhost:{port}/info').status_code == 200

        await _wait_for(lambda: _workers_in(_scrape(metrics_port)[1], 'requests_handled') == {'1': requests})
        await _wait_for(lambda: _workers_in(_scrape(metrics_port)[1], 'connections_active') == {'1': 0})
        _, samples = _scrape(metrics_port)

    assert samples[('connections_handled', '1')] >= 1
    assert samples[('connections_err', '1')] == 0
    assert samples[('blocking_threads', '1')] >= 1
    assert samples[('blocking_queue', '1')] == 0
    if interface == 'wsgi':
        # WSGI runs every request on the blocking pool
        assert samples[('blocking_busy_cumulative', '1')] > 0


async def test_blocking_queue_never_negative(server_port):
    async with _metrics_server('wsgi', server_port) as (_, port, metrics_port):
        observed = []
        async with httpx.AsyncClient() as client:

            async def load():
                for _ in range(20):
                    await asyncio.gather(*(client.get(f'http://localhost:{port}/info') for _ in range(10)))

            task = asyncio.create_task(load())
            while not task.done():
                observed.append(_scrape(metrics_port)[1][('blocking_queue', '1')])
                await asyncio.sleep(0.05)
            await task
        await _wait_for(lambda: _scrape(metrics_port)[1][('blocking_queue', '1')] == 0)

    assert min(observed) >= 0


@pytest.mark.skipif(not BUILD_GIL, reason='RSS monitor not available on free-threaded builds')
async def test_respawn_keeps_newest_worker(server_port):
    requests = 20
    # a 1MiB RSS limit forces a graceful respawn on every sample; the fractional
    # interval offsets old/new worker reports by ~0.5s, so a stale report from
    # the old worker would stay visible long enough to be scraped.
    async with _metrics_server('asgi', server_port, workers_max_rss=1, rss_sample_interval=2.5, respawn_interval=4) as (
        _,
        port,
        metrics_port,
    ):
        with httpx.Client() as client:
            for _ in range(requests):
                client.get(f'http://localhost:{port}/info')
        await _wait_for(lambda: _scrape(metrics_port)[1][('requests_handled', '1')] == requests)

        # sample through the old/new worker overlap window
        history = []
        deadline = time.monotonic() + 12
        while time.monotonic() < deadline:
            samples = _try_scrape(metrics_port)
            if ('requests_handled', '1') in samples:
                history.append(samples[('requests_handled', '1')])
            await asyncio.sleep(0.1)
        _, samples = _scrape(metrics_port)

    assert samples[('workers_respawns_for_rss', None)] >= 1
    assert any(value < requests for value in history), f'no respawn observed: {history}'
    first_drop = next(idx for idx, value in enumerate(history) if value < requests)
    # once the new worker reports, the old worker must never show up again
    assert all(value < requests for value in history[first_drop:]), history


@pytest.mark.skipif(not BUILD_GIL, reason='workers are threads on free-threaded builds')
async def test_crashed_worker_is_cleared(server_port):
    async with _metrics_server(
        'asgi', server_port, workers=2, respawn_failed_workers=True, metrics_scrape_interval=3
    ) as (proc, _, metrics_port):
        pids = _worker_pids(proc)
        assert len(pids) == 2
        before = _workers_in(_scrape(metrics_port)[1], 'requests_handled')
        assert set(before) == {'1', '2'}

        os.kill(pids[0], signal.SIGKILL)
        # the dead worker's series goes away before the replacement reports
        await _wait_for(lambda: len(_workers_in(_scrape(metrics_port)[1], 'requests_handled')) == 1, timeout=3)
        await _wait_for(lambda: len(_workers_in(_scrape(metrics_port)[1], 'requests_handled')) == 2, timeout=10)
        _, samples = _scrape(metrics_port)

    assert samples[('workers_respawns_for_err', None)] == 1
    assert samples[('workers_spawns', None)] == 3


async def test_simultaneous_crashes_counted_per_worker():
    server = Granian('tests.apps.asgi:app', workers=2, respawn_failed_workers=True)
    server._metrics = mock.Mock()

    def respawn(*args, **kwargs):
        # stop the loop after the first respawn cycle
        server.interrupt_signal = True
        server.main_loop_interrupt.set()

    server._respawn_workers = mock.Mock(side_effect=respawn)
    # both workers died before the main loop woke up
    server.interrupt_children.extend([0, 1])
    server.main_loop_interrupt.set()
    server._serve_loop(None, None)

    assert server._respawn_workers.call_args.args[0] == [0, 1]
    server._metrics.incr_respawn_err.assert_called_once_with(2)
    assert server._metrics.clear.call_args_list == [mock.call(0), mock.call(1)]
