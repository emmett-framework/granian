import json

from granian.rsgi import (
    HTTPProtocol,
    Scope,
    WebsocketMessageType,
    WebsocketProtocol
)


async def info(scope: Scope, protocol: HTTPProtocol):
    protocol.response_bytes(
        200,
        [('content-type', 'application/json')],
        json.dumps({
            'proto': scope.proto,
            'http_version': scope.http_version,
            'rsgi_version': scope.rsgi_version,
            'scheme': scope.scheme,
            'method': scope.method,
            'path': scope.path,
            'query_string': scope.query_string,
            'headers': {k: v for k, v in scope.headers.items()}
        }).encode("utf8")
    )


async def echo(_, protocol: HTTPProtocol):
    msg = await protocol()
    protocol.response_bytes(
        200,
        [('content-type', 'text/plain; charset=utf-8')],
        msg
    )


async def ws_reject(_, protocol: WebsocketProtocol):
    protocol.close(403)


async def ws_info(scope: Scope, protocol: WebsocketProtocol):
    trx = await protocol.accept()

    await trx.send_str(json.dumps({
        'proto': scope.proto,
        'http_version': scope.http_version,
        'rsgi_version': scope.rsgi_version,
        'scheme': scope.scheme,
        'method': scope.method,
        'path': scope.path,
        'query_string': scope.query_string,
        'headers': {k: v for k, v in scope.headers.items()}
    }))
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


async def err_app(scope: Scope, protocol: HTTPProtocol):
    1 / 0


def app(scope, protocol):
    return {
        "/info": info,
        "/echo": echo,
        "/ws_reject": ws_reject,
        "/ws_info": ws_info,
        "/ws_echo": ws_echo,
        "/err_app": err_app
    }[scope.path](scope, protocol)
