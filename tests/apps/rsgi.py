import json
import os
import tempfile
from pathlib import Path
from urllib.parse import parse_qs

from granian.rsgi import HTTPProtocol, Scope, WebsocketMessageType, WebsocketProtocol


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

    # Create test file
    with tempfile.NamedTemporaryFile(mode='wb', suffix='.bin', delete=False) as tmp:
        tmp.write(b'0123456789' * 10)  # 100 bytes
        file_path = tmp.name

    try:
        # Parse query parameters using urllib.parse
        params = parse_qs(scope.query_string) if scope.query_string else {}

        # Extract start and end from params (parse_qs returns lists)
        start = int(params.get('start', ['0'])[0])
        end = int(params.get('end', ['9'])[0])

        # Check if this is an error test
        if 'test_error' in params:
            try:
                protocol.response_file_partial(206, [], file_path, start, end)
                protocol.response_empty(500, [])  # Should not reach here if error expected
            except ValueError as e:
                protocol.response_str(400, [('content-type', 'text/plain')], str(e))
            return

        # Normal response
        headers = [
            ('content-type', 'text/plain'),
            ('content-range', f'bytes {start}-{end}/100'),
            ('content-length', str(end - start + 1)),
        ]

        protocol.response_file_partial(206, headers, file_path, start, end)
    finally:
        # Clean up temp file
        try:
            os.unlink(file_path)
        except:  # noqa: E722
            pass


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
