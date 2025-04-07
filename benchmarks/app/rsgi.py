import asyncio
import pathlib
import sys


HEADERS = [('content-type', 'text/plain; charset=utf-8')]
HEADERS_MEDIA = [('content-type', 'image/jpeg'), ('content-length', '50486')]

BODY_BYTES = {
    10: b'x' * 10,
    1000: b'x' * 1024,
    10_000: b'x' * 1024 * 10,
    100_000: b'x' * 1024 * 100,
}
BODY_STR = {
    10: 'x' * 10,
    1000: 'x' * 1024,
    10_000: 'x' * 1024 * 10,
    100_000: 'x' * 1024 * 100,
}

MEDIA_PATH = str(pathlib.Path(__file__).parent / 'assets' / 'media.jpg')


def b_builder(size):
    body = BODY_BYTES[size]

    async def route(scope, proto):
        proto.response_bytes(200, HEADERS, body)

    return route


def s_builder(size):
    body = BODY_STR[size]

    async def route(scope, proto):
        proto.response_str(200, HEADERS, body)

    return route


async def echo(scope, proto):
    proto.response_bytes(200, HEADERS, await proto())


async def echo_iter(scope, proto):
    trx = proto.response_stream(200, HEADERS)
    async for chunk in proto:
        await trx.send_bytes(chunk)


async def file(scope, proto):
    proto.response_file(200, HEADERS_MEDIA, MEDIA_PATH)


def io_builder(wait):
    wait = wait / 1000

    async def io(scope, proto):
        await asyncio.sleep(wait)
        proto.response_bytes(200, HEADERS, BODY_BYTES[10])

    return io


async def handle_404(scope, proto):
    proto.response_str(404, HEADERS, 'not found')


routes = {
    '/b10': b_builder(10),
    '/b1k': b_builder(1000),
    '/b10k': b_builder(10_000),
    '/b100k': b_builder(100_000),
    '/s10': s_builder(10),
    '/s1k': s_builder(1000),
    '/s10k': s_builder(10_000),
    '/s100k': s_builder(100_000),
    '/echo': echo,
    '/echoi': echo_iter,
    '/fp': file,
    '/io10': io_builder(10),
    '/io100': io_builder(100),
}


def app(scope, proto):
    handler = routes.get(scope.path, handle_404)
    return handler(scope, proto)


def granian(wrk, thr):
    from granian import Granian

    Granian('rsgi:app', workers=int(wrk), threads=int(thr), interface='rsgi').serve()


if __name__ == '__main__':
    granian(sys.argv[1], sys.argv[2])
