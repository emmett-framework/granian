import json
import os
import pathlib

from granian.rsgi import AsyncIORSGIHTTPProtocol, AsyncIORSGIWebsocketProtocol, Scope, WebsocketMessageType


async def info(scope: Scope, protocol: AsyncIORSGIHTTPProtocol):
    protocol.write_bytes(
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


async def echo(_, protocol: AsyncIORSGIHTTPProtocol):
    msg = await protocol.read()
    protocol.write_bytes(200, [('content-type', 'text/plain; charset=utf-8')], msg)


async def echo_stream(_, protocol: AsyncIORSGIHTTPProtocol):
    trx = protocol.writer(200, [('content-type', 'text/plain; charset=utf-8')])
    async for msg in protocol.reader():
        await trx.write_bytes(msg)


async def stream(_, protocol: AsyncIORSGIHTTPProtocol):
    trx = protocol.writer(200, [('content-type', 'text/plain; charset=utf-8')])
    for _ in range(0, 3):
        await trx.write_bytes(b'test')


async def file(scope: Scope, protocol: AsyncIORSGIHTTPProtocol):
    path = pathlib.Path.cwd() / 'tests' / 'fixtures' / 'static' / 'media.png'
    protocol.write_file(200, [('content-type', 'image/png'), ('content-length', '95')], str(path))


async def file_range(scope: Scope, protocol: AsyncIORSGIHTTPProtocol):
    file_path = scope.headers.get('file-path')
    range_header: str = scope.headers.get('range')
    start, end = [int(v) for v in range_header.removeprefix('bytes=').split('-')]
    file_size = os.stat(file_path).st_size
    if start >= file_size:
        return protocol.write(416, [('content-range', f'bytes */{file_size}'), ('4xx-reason', 'out')])
    if end >= file_size:
        end = file_size - 1

    headers = [
        ('content-type', 'text/plain'),
        ('content-length', f'{end - start + 1}'),
        ('content-range', f'bytes {start}-{end}/{file_size}'),
    ]
    try:
        protocol.write_file_range(206, headers, file_path, start, end + 1)
    except ValueError:
        protocol.write(416, [('content-range', f'bytes */{file_size}'), ('4xx-reason', 'invalid')])


async def ws_reject(_, protocol: AsyncIORSGIWebsocketProtocol):
    protocol.close(403)


async def ws_info(scope: Scope, protocol: AsyncIORSGIWebsocketProtocol):
    rx, tx = await protocol.accept()

    await tx.write_str(
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
        message = await rx.read()
        if message.kind == WebsocketMessageType.close:
            break

    protocol.close()


async def ws_echo(_, protocol: AsyncIORSGIWebsocketProtocol):
    rx, tx = await protocol.accept()

    while True:
        message = await rx.read()
        if message.kind == WebsocketMessageType.close:
            break
        elif message.kind == WebsocketMessageType.bytes:
            await tx.write_bytes(message.data)
        else:
            await tx.write_str(message.data)

    protocol.close()


async def ws_push(_, protocol: AsyncIORSGIWebsocketProtocol):
    _rx, tx = await protocol.accept()

    try:
        while True:
            await tx.write_str('ping')
    except Exception:
        pass

    protocol.close()


async def err_app(scope: Scope, protocol: AsyncIORSGIHTTPProtocol):
    1 / 0


def app(scope, protocol):
    return {
        '/info': info,
        '/echo': echo,
        '/echos': echo_stream,
        '/file': file,
        '/file_range': file_range,
        '/stream': stream,
        '/ws_reject': ws_reject,
        '/ws_info': ws_info,
        '/ws_echo': ws_echo,
        '/ws_push': ws_push,
        '/err_app': err_app,
    }[scope.path](scope, protocol)
