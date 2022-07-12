import json

PLAINTEXT_RESPONSE = {
    'type': 'http.response.start',
    'status': 200,
    'headers': [
        [b'content-type', b'text/plain; charset=utf-8'],
    ]
}
JSON_RESPONSE = {
    'type': 'http.response.start',
    'status': 200,
    'headers': [
        [b'content-type', b'application/json'],
    ]
}


async def info(scope, receive, send):
    await send(JSON_RESPONSE)
    await send({
        'type': 'http.response.body',
        'body': json.dumps({
            'type': scope['type'],
            'asgi': scope['asgi'],
            'http_version': scope['http_version'],
            'scheme': scope['scheme'],
            'method': scope['method'],
            'path': scope['path'],
            'query_string': scope['query_string'].decode("latin-1"),
            'headers': {
                k.decode("utf8"): v.decode("utf8")
                for k, v in scope['headers'].items()
            }
        }).encode("utf8"),
        'more_body': False
    })


async def echo(scope, receive, send):
    await send(PLAINTEXT_RESPONSE)
    msg = await receive()
    await send({
        'type': 'http.response.body',
        'body': msg['body'],
        'more_body': False
    })


def app(scope, receive, send):
    return {
        "/info": info,
        "/echo": echo
    }[scope['path']](scope, receive, send)
