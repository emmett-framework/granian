import asyncio
import pathlib
import sys


HEADERS = [('content-type', 'text/plain; charset=utf-8')]
HEADERS_MEDIA = [('content-type', 'image/png'), ('content-length', '95')]

BODY_BYTES_SHORT = b'Test'
BODY_BYTES_LONG = b'Test' * 20_000
BODY_STR_SHORT = 'Test'
BODY_STR_LONG = 'Test' * 20_000

MEDIA_PATH = str(pathlib.Path(__file__).parent.parent / 'files' / 'media.png')


async def b_short(scope, proto):
    proto.response_bytes(200, HEADERS, BODY_BYTES_SHORT)


async def b_long(scope, proto):
    proto.response_bytes(200, HEADERS, BODY_BYTES_LONG)


async def s_short(scope, proto):
    proto.response_str(200, HEADERS, BODY_STR_SHORT)


async def s_long(scope, proto):
    proto.response_str(200, HEADERS, BODY_STR_LONG)


async def echo(scope, proto):
    proto.response_bytes(200, HEADERS, await proto())


async def file(scope, proto):
    proto.response_file(200, HEADERS_MEDIA, MEDIA_PATH)


def io_builder(wait):
    wait = wait / 1000

    async def io(scope, proto):
        await asyncio.sleep(wait)
        proto.response_bytes(200, HEADERS, BODY_BYTES_SHORT)

    return io


async def handle_404(scope, proto):
    proto.response_str(404, HEADERS, 'not found')


routes = {
    '/b': b_short,
    '/bb': b_long,
    '/s': s_short,
    '/ss': s_long,
    '/echo': echo,
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
