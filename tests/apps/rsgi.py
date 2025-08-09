import json
import os
import tempfile
from pathlib import Path
from urllib.parse import parse_qs

from granian.rsgi import HTTPProtocol, Scope, WebsocketMessageType, WebsocketProtocol


# Pre-create test file for partial serving tests
_TEST_FILE_PATH = None


def _get_test_file():
    """Get or create the test file for partial serving."""
    global _TEST_FILE_PATH
    if _TEST_FILE_PATH is None:
        fd, _TEST_FILE_PATH = tempfile.mkstemp(suffix='.bin', prefix='granian_partial_test_')
        with os.fdopen(fd, 'wb') as f:
            f.write(b'0123456789' * 10)  # 100 bytes
    return _TEST_FILE_PATH


async def info(scope: Scope, protocol: HTTPProtocol):
    protocol.response_bytes(
        200,
        [('content-type', 'application/json')],
        json.dumps(
            {
                'proto': scope.proto,
                'http_version': scope.http_version,
                'rsgi_version': scope.rsgi_version,
                'scheme': scope.scheme,
                'method': scope.method,
                'path': scope.path,
                'query_string': scope.query_string,
                'headers': dict(scope.headers.items()),
                'authority': scope.authority,
            }
        ).encode('utf8'),
    )


async def echo(_, protocol: HTTPProtocol):
    msg = await protocol()
    protocol.response_bytes(200, [('content-type', 'text/plain; charset=utf-8')], msg)


async def echo_stream(_, protocol: HTTPProtocol):
    trx = protocol.response_stream(200, [('content-type', 'text/plain; charset=utf-8')])
    async for msg in protocol:
        await trx.send_bytes(msg)


async def stream(_, protocol: HTTPProtocol):
    trx = protocol.response_stream(200, [('content-type', 'text/plain; charset=utf-8')])
    for _ in range(0, 3):
        await trx.send_bytes(b'test')


async def ws_reject(_, protocol: WebsocketProtocol):
    protocol.close(403)


async def ws_info(scope: Scope, protocol: WebsocketProtocol):
    trx = await protocol.accept()

    await trx.send_str(
        json.dumps(
            {
                'proto': scope.proto,
                'http_version': scope.http_version,
                'rsgi_version': scope.rsgi_version,
                'scheme': scope.scheme,
                'method': scope.method,
                'path': scope.path,
                'query_string': scope.query_string,
                'headers': dict(scope.headers.items()),
                'authority': scope.authority,
            }
        )
    )
    while True:
        message = await trx.receive()
        if message.kind == WebsocketMessageType.close:
            break

    protocol.close()


async def ws_echo(_, protocol: WebsocketProtocol):
    trx = await protocol.accept()

    while True:
        message = await trx.receive()
        if message.kind == WebsocketMessageType.close:
            break
        elif message.kind == WebsocketMessageType.bytes:
            await trx.send_bytes(message.data)
        else:
            await trx.send_str(message.data)

    protocol.close()


async def ws_push(_, protocol: WebsocketProtocol):
    trx = await protocol.accept()

    try:
        while True:
            await trx.send_str('ping')
    except Exception:
        pass

    protocol.close()


async def err_app(scope: Scope, protocol: HTTPProtocol):
    1 / 0


async def serve_file(scope: Scope, protocol: HTTPProtocol):
    file_path = Path(os.environ.get('ROOT_PATH', '.'), 'test.txt')
    protocol.response_file(200, [('content-type', 'text/plain; charset=utf-8')], str(file_path))


async def serve_file_partial(scope: Scope, protocol: HTTPProtocol):
    """Simple test handler for response_file_partial - uses query parameters"""
    file_path = _get_test_file()

    params = parse_qs(scope.query_string) if scope.query_string else {}
    start = int(params.get('start', ['0'])[0])
    end = int(params.get('end', ['9'])[0])

    protocol.response_file_partial(206, [('content-type', 'text/plain')], file_path, start, end)


def app(scope, protocol):
    return {
        '/info': info,
        '/echo': echo,
        '/echos': echo_stream,
        '/file': serve_file,
        '/file_partial': serve_file_partial,
        '/stream': stream,
        '/ws_reject': ws_reject,
        '/ws_info': ws_info,
        '/ws_echo': ws_echo,
        '/ws_push': ws_push,
        '/err_app': err_app,
    }[scope.path](scope, protocol)
