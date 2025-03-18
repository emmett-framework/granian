import asyncio
import os

import httpx
import pytest

from granian._granian import BUILD_GIL
from granian.server.embed import Server as EmbeddedGranian


async def app(scope, protocol):
    protocol.response_str(200, [], 'hello')


@pytest.fixture(scope='function')
def loop():
    return asyncio.get_event_loop()


@pytest.fixture(scope='function')
def embed_server(server_port):
    return EmbeddedGranian(app, port=server_port)


@pytest.mark.skipif(not BUILD_GIL, reason='free-threaded Python')
@pytest.mark.skipif(bool(os.environ.get('GITHUB_WORKFLOW')), reason='CI')
def test_embed_server(loop, server_port, embed_server):
    data = {}

    async def client():
        await asyncio.sleep(1.5)

        h = httpx.AsyncClient()
        try:
            data['res'] = await h.get(f'http://localhost:{server_port}')
        finally:
            embed_server.stop()

    server_task = loop.create_task(embed_server.serve())
    loop.run_until_complete(client())

    assert data['res'].status_code == 200
    assert data['res'].text == 'hello'

    loop.run_until_complete(server_task)
