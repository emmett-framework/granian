import asyncio
import os

import httpx
import pytest
import websockets

from granian import Granian
from granian.constants import Schedulers


PGO_RUN = bool(os.getenv('PGO_RUN'))


def test_scheduler_defaults_to_fifo():
    server = Granian('tests.apps.asgi:app')

    assert server.scheduler is Schedulers.fifo
    assert server.fq_config is None


def test_fair_scheduler_config():
    server = Granian('tests.apps.asgi:app', scheduler='fair')

    assert server.scheduler is Schedulers.fair
    assert server.fq_config is not None
    assert server.fq_config['outer_prefix_v4'] == 24
    assert server.fq_config['outer_prefix_v6'] == 56
    assert server.fq_config['inner_headers'][0] == 'user-agent'
    assert server.fq_config['inner_cookies'] == ['sessionid', 'session']
    assert server.fq_config['inner_include_ip'] is True
    assert server.fq_config['monitoring_tool_id'] == 4


def test_fair_scheduler_config_overrides():
    server = Granian(
        'tests.apps.asgi:app',
        scheduler='fair',
        fq_outer_prefix_v4=16,
        fq_inner_max=128,
        fq_burst=50,
        fq_max_debt=1000,
        fq_debt_ttl=5,
        fq_trusted_proxies='*',
        fq_inner_include_ip=False,
        fq_monitoring_tool_id=3,
    )

    assert server.fq_config['outer_prefix_v4'] == 16
    assert server.fq_config['inner_max'] == 128
    assert server.fq_config['burst_ms'] == 50
    assert server.fq_config['max_debt_ms'] == 1000
    assert server.fq_config['debt_ttl_secs'] == 5
    assert server.fq_config['trusted_proxies'] == ['*']
    assert server.fq_config['inner_include_ip'] is False
    assert server.fq_config['monitoring_tool_id'] == 3


@pytest.mark.asyncio
@pytest.mark.skipif(PGO_RUN, reason='PGO build')
@pytest.mark.parametrize('server', ['asgi', 'rsgi', 'wsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_fair_scheduler_serves_requests(server, server_port, runtime_mode):
    async with server(runtime_mode, scheduler='fair') as port:
        res = httpx.get(f'http://localhost:{port}/info')

    assert res.status_code == 200


@pytest.mark.asyncio
@pytest.mark.skipif(PGO_RUN, reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_fair_scheduler_handles_concurrency(asgi_server, runtime_mode):
    async with asgi_server(runtime_mode, scheduler='fair') as port:
        async with httpx.AsyncClient() as client:
            results = await asyncio.gather(
                *(
                    client.get(f'http://localhost:{port}/info', headers={'user-agent': f'browser-{idx % 4}'})
                    for idx in range(50)
                )
            )

    assert all(res.status_code == 200 for res in results)


@pytest.mark.asyncio
@pytest.mark.skipif(PGO_RUN, reason='PGO build')
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_fair_scheduler_wsgi_streaming(wsgi_server, runtime_mode):
    async with wsgi_server(runtime_mode, scheduler='fair') as port:
        res = httpx.get(f'http://localhost:{port}/iterbody')

    assert res.status_code == 200
    assert res.text == 'testtesttest'


@pytest.mark.asyncio
@pytest.mark.skipif(PGO_RUN, reason='PGO build')
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_fair_scheduler_websockets(server, runtime_mode):
    async with server(runtime_mode, scheduler='fair') as port:
        async with websockets.connect(f'ws://localhost:{port}/ws_echo') as ws:
            await ws.send('foo')
            res_text = await ws.recv()
            await ws.send(b'foo')
            res_bytes = await ws.recv()

    assert res_text == 'foo'
    assert res_bytes == b'foo'
