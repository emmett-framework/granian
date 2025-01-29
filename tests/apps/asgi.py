import asyncio
import json
import pathlib

import sniffio


PLAINTEXT_RESPONSE = {
    'type': 'http.response.start',
    'status': 200,
    'headers': [[b'content-type', b'text/plain; charset=utf-8']],
}
JSON_RESPONSE = {'type': 'http.response.start', 'status': 200, 'headers': [[b'content-type', b'application/json']]}
MEDIA_RESPONSE = {
    'type': 'http.response.start',
    'status': 200,
    'headers': [[b'content-type', b'image/png'], [b'content-length', b'95']],
}


async def info(scope, receive, send):
    await send(JSON_RESPONSE)
    await send(
        {
            'type': 'http.response.body',
            'body': json.dumps(
                {
                    'type': scope['type'],
                    'asgi': scope['asgi'],
                    'http_version': scope['http_version'],
                    'scheme': scope['scheme'],
                    'method': scope['method'],
                    'path': scope['path'],
                    'query_string': scope['query_string'].decode('latin-1'),
                    'headers': {k.decode('utf8'): v.decode('utf8') for k, v in scope['headers']},
                    'extensions': scope['extensions'],
                    'state': scope['state'],
                }
            ).encode('utf8'),
            'more_body': False,
        }
    )


async def sniff_aio_impl(scope, receive, send):
    await send(PLAINTEXT_RESPONSE)
    await send(
        {'type': 'http.response.body', 'body': sniffio.current_async_library().encode('utf8'), 'more_body': False}
    )


async def echo(scope, receive, send):
    await send(PLAINTEXT_RESPONSE)
    more_body = True
    body = b''
    while more_body:
        msg = await receive()
        more_body = msg['more_body']
        body += msg['body']
    await send({'type': 'http.response.body', 'body': body, 'more_body': False})


async def pathsend(scope, receive, send):
    path = pathlib.Path.cwd() / 'tests' / 'fixtures' / 'media.png'
    await send(MEDIA_RESPONSE)
    await send({'type': 'http.response.pathsend', 'path': str(path)})


async def ws_reject(scope, receive, send):
    return


async def ws_info(scope, receive, send):
    await send({'type': 'websocket.accept'})
    await send(
        {
            'type': 'websocket.send',
            'text': json.dumps(
                {
                    'type': scope['type'],
                    'asgi': scope['asgi'],
                    'http_version': scope['http_version'],
                    'scheme': scope['scheme'],
                    'path': scope['path'],
                    'query_string': scope['query_string'].decode('latin-1'),
                    'headers': {k.decode('utf8'): v.decode('utf8') for k, v in scope['headers']},
                    'subprotocols': scope['subprotocols'],
                }
            ),
        }
    )
    await send({'type': 'websocket.close'})


async def ws_echo(scope, receive, send):
    await send({'type': 'websocket.accept'})

    while True:
        msg = await receive()
        if msg['type'] == 'websocket.connect':
            continue
        if msg['type'] == 'websocket.disconnect':
            break
        rv = {'type': 'websocket.send', 'bytes': None, 'text': None}
        key = 'text' if 'text' in msg else 'bytes'
        rv[key] = msg[key]
        await send(rv)

    await send({'type': 'websocket.close'})


async def ws_push(scope, receive, send):
    await send({'type': 'websocket.accept'})

    try:
        while True:
            await send({'type': 'websocket.send', 'text': 'ping'})
    except Exception:
        pass


async def err_app(scope, receive, send):
    1 / 0


async def err_proto(scope, receive, send):
    await send({'type': 'wrong.msg'})


async def timeout_n(scope, receive, send):
    async def _inner():
        return b'ok'

    await send(PLAINTEXT_RESPONSE)
    try:
        ret = await asyncio.wait_for(_inner(), None)
    except asyncio.TimeoutError:
        ret = b'timeout'
    await send({'type': 'http.response.body', 'body': ret, 'more_body': False})


async def timeout_w(scope, receive, send):
    async def _inner():
        await asyncio.sleep(3)
        return b'ok'

    await send(PLAINTEXT_RESPONSE)
    try:
        ret = await asyncio.wait_for(_inner(), 1)
    except asyncio.TimeoutError:
        ret = b'timeout'
    await send({'type': 'http.response.body', 'body': ret, 'more_body': False})


async def lifespan(scope, receive, send):
    msg = await receive()
    if msg['type'] == 'lifespan.startup':
        scope['state']['global'] = 'test'
        await send({'type': 'lifespan.startup.complete'})


def app(scope, receive, send):
    if scope['type'] == 'lifespan':
        return lifespan(scope, receive, send)
    return {
        '/info': info,
        '/sniffio': sniff_aio_impl,
        '/echo': echo,
        '/file': pathsend,
        '/ws_reject': ws_reject,
        '/ws_info': ws_info,
        '/ws_echo': ws_echo,
        '/ws_push': ws_push,
        '/err_app': err_app,
        '/err_proto': err_proto,
        '/timeout_n': timeout_n,
        '/timeout_w': timeout_w,
    }.get(scope['path'], info)(scope, receive, send)
