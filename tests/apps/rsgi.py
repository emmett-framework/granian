import json

from granian.rsgi import Response


async def info(scope, transport):
    return Response.bytes(
        json.dumps({
            'proto': scope.proto,
            'http_version': scope.http_version,
            'scheme': scope.scheme,
            'method': scope.method,
            'path': scope.path,
            'query_string': scope.query_string,
            'headers': {k: v for k, v in scope.headers.items()}
        }).encode("utf8"),
        headers={'content-type': 'application/json'}
    )


async def echo(scope, transport):
    msg = await transport()
    return Response.bytes(
        msg,
        headers={'content-type': 'text/plain; charset=utf-8'}
    )


async def ws_reject(scope, transport):
    return transport.close(403)


async def ws_echo(scope, transport):
    proto = await transport.accept()

    while True:
        message = await proto.receive()
        if message.kind == 0:
            break
        elif message.kind == 1:
            await proto.send_bytes(message.data)
        else:
            await proto.send_str(message.data)

    return transport.close()


def app(scope, transport):
    return {
        "/info": info,
        "/echo": echo,
        "/ws_reject": ws_reject,
        "/ws_echo": ws_echo
    }[scope.path](scope, transport)
