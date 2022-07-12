import httpx
import pytest


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "threading_mode",
    [
        "runtime",
        "workers"
    ]
)
async def test_scope(rsgi_server, threading_mode):
    async with rsgi_server(threading_mode) as port:
        res = httpx.get(f"http://localhost:{port}/info?test=true")

    assert res.status_code == 200
    data = res.json()
    assert data['proto'] == "http"
    assert data['http_version'] == '1.1'
    assert data['scheme'] == 'http'
    assert data['method'] == "GET"
    assert data['path'] == '/info'
    assert data['query_string'] == 'test=true'
    assert data['headers']['host'] == f'localhost:{port}'


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "threading_mode",
    [
        "runtime",
        "workers"
    ]
)
async def test_body(rsgi_server, threading_mode):
    async with rsgi_server(threading_mode) as port:
        res = httpx.post(f"http://localhost:{port}/echo", data="test")

    assert res.status_code == 200
    assert res.text == "test"
