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
async def test_scope(asgi_server, threading_mode):
    async with asgi_server(threading_mode):
        res = httpx.get("http://localhost:8000/info?test=true")

    assert res.status_code == 200
    data = res.json()
    assert data['asgi'] == {
        'version': '3.0',
        'spec_version': '2.3'
    }
    assert data['type'] == "http"
    assert data['http_version'] == '1.1'
    assert data['scheme'] == 'http'
    assert data['method'] == "GET"
    assert data['path'] == '/info'
    assert data['query_string'] == 'test=true'
    assert data['headers']['host'] == 'localhost:8000'


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "threading_mode",
    [
        "runtime",
        "workers"
    ]
)
async def test_body(asgi_server, threading_mode):
    async with asgi_server(threading_mode):
        res = httpx.post("http://localhost:8000/echo", data="test")

    assert res.status_code == 200
    assert res.text == "test"
