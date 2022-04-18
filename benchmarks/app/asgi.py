import sys

PLAINTEXT_RESPONSE = {
    'type': 'http.response.start',
    'status': 200,
    'headers': [
        [b'content-type', b'text/plain; charset=utf-8'],
    ]
}

BODY_BYTES_SHORT = b"Test"
BODY_BYTES_LONG = b"Test" * 20_000
BODY_STR_SHORT = "Test"
BODY_STR_LONG = "Test" * 20_000


async def b_short(scope, receive, send):
    await send(PLAINTEXT_RESPONSE)
    await send({
        'type': 'http.response.body',
        'body': BODY_BYTES_SHORT,
        'more_body': False
    })


async def b_long(scope, receive, send):
    await send(PLAINTEXT_RESPONSE)
    await send({
        'type': 'http.response.body',
        'body': BODY_BYTES_LONG,
        'more_body': False
    })


async def s_short(scope, receive, send):
    await send(PLAINTEXT_RESPONSE)
    await send({
        'type': 'http.response.body',
        'body': BODY_STR_SHORT.encode("utf8"),
        'more_body': False
    })


async def s_long(scope, receive, send):
    await send(PLAINTEXT_RESPONSE)
    await send({
        'type': 'http.response.body',
        'body': BODY_STR_LONG.encode("utf8"),
        'more_body': False
    })


async def handle_404(scope, receive, send):
    content = b'Not found'
    await send(PLAINTEXT_RESPONSE)
    await send({
        'type': 'http.response.body',
        'body': content,
        'more_body': False
    })


routes = {
    '/b': b_short,
    '/bb': b_long,
    '/s': s_short,
    '/ss': s_long
}


def app(scope, receive, send):
    handler = routes.get(scope['path'], handle_404)
    return handler(scope, receive, send)


def granian(wrk, thr):
    from granian import Granian
    Granian("asgi:app", workers=int(wrk), threads=int(thr), interface="asgi").serve()


if __name__ == "__main__":
    granian(sys.argv[1], sys.argv[2])
