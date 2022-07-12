import json

from granian.rsgi import Response


async def info(scope, receive):
    return Response(
        1, 200, {'content-type': 'application/json'}, json.dumps({
            'proto': scope.proto,
            'http_version': scope.http_version,
            'scheme': scope.scheme,
            'method': scope.method,
            'path': scope.path,
            'query_string': scope.query_string,
            'headers': {k: v for k, v in scope.headers.items()}
        }).encode("utf8"), None, None
    )


async def echo(scope, receive):
    msg = await receive()
    return Response(
        1, 200, {'content-type': 'text/plain; charset=utf-8'}, msg, None, None
    )


def app(scope, receive):
    return {
        "/info": info,
        "/echo": echo
    }[scope.path](scope, receive)
