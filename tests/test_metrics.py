import socket
from contextlib import closing

import httpx
import pytest


def get_free_port():
    with closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as sock:
        sock.bind(('localhost', 0))
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        return sock.getsockname()[1]


@pytest.mark.asyncio
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_metrics_content_type(asgi_server, runtime_mode):
    metrics_port = get_free_port()
    async with asgi_server(
        runtime_mode,
        metrics_enabled=True,
        metrics_port=metrics_port
    ):
        async with httpx.AsyncClient() as client:
            res = await client.get(f'http://localhost:{metrics_port}/metrics')

    assert res.status_code == 200
    assert res.headers['content-type'] == 'text/plain; version=0.0.4; charset=utf-8'
