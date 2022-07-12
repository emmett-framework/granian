import asyncio
import os

from contextlib import asynccontextmanager
from functools import partial

import pytest


@asynccontextmanager
async def server(interface, threading_mode):
    proc = await asyncio.create_subprocess_shell(
        f"granian --interface {interface} --threads 1 "
        f"--threading-mode {threading_mode} "
        f"tests.apps.{interface}:app",
        env=dict(os.environ)
    )
    await asyncio.sleep(0.5)
    try:
        yield
    finally:
        proc.terminate()
        await proc.wait()


@pytest.fixture(scope="function")
def asgi_server():
    return partial(server, "asgi")


@pytest.fixture(scope="function")
def rsgi_server():
    return partial(server, "rsgi")
