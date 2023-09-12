import httpx
import pytest
import asyncio

async def make_req(port):
    async with httpx.AsyncClient() as client:
        return await client.post(f"http://localhost:{port}/echo", content="test")

@pytest.mark.asyncio
@pytest.mark.parametrize("threading_mode", ["runtime", "workers"])
@pytest.mark.parametrize("server_mode", ["wsgi", "asgi", "rsgi"])
async def test_lock(wsgi_server, asgi_server, rsgi_server, threading_mode, server_mode):
    server = {"wsgi": wsgi_server,
              "asgi": asgi_server,
              "rsgi": rsgi_server,
              }[server_mode]
    async with server(threading_mode, workers=2) as port:
        for _ in range(100):
            ok = 0
            timeout = 0
            for res in await asyncio.gather(*[make_req(port) for _ in range(3)], return_exceptions=True):
                if isinstance(res, Exception):
                    timeout += 1
                else:
                    assert res.status_code == 200
                    ok += 1
            assert (timeout, ok) == (0, 3)
