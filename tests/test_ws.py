import pytest
import websockets


@pytest.mark.asyncio
@pytest.mark.parametrize("server", ["asgi", "rsgi"], indirect=True)
@pytest.mark.parametrize("threading_mode", ["runtime", "workers"])
async def test_messages(server, threading_mode):
    async with server(threading_mode) as port:
        async with websockets.connect(f"ws://localhost:{port}/ws_echo") as ws:
            await ws.send("foo")
            res_text = await ws.recv()
            await ws.send(b"foo")
            res_bytes = await ws.recv()

    assert res_text == "foo"
    assert res_bytes == b"foo"


@pytest.mark.asyncio
@pytest.mark.parametrize("server", ["asgi", "rsgi"], indirect=True)
@pytest.mark.parametrize("threading_mode", ["runtime", "workers"])
async def test_reject(server, threading_mode):
    async with server(threading_mode) as port:
        with pytest.raises(websockets.InvalidStatusCode) as exc:
            async with websockets.connect(f"ws://localhost:{port}/ws_reject") as ws:
                pass

    assert exc.value.status_code == 403
