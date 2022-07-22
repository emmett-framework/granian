import sys

from granian.rsgi import Response

HEADERS = {'content-type': 'text/plain; charset=utf-8'}

BODY_BYTES_SHORT = b"Test"
BODY_BYTES_LONG = b"Test" * 20_000
BODY_STR_SHORT = "Test"
BODY_STR_LONG = "Test" * 20_000


async def b_short(scope, proto):
    return Response.bytes(BODY_BYTES_SHORT, 200, HEADERS)


async def b_long(scope, proto):
    return Response.bytes(BODY_BYTES_LONG, 200, HEADERS)


async def s_short(scope, proto):
    return Response.str(BODY_STR_SHORT, 200, HEADERS)


async def s_long(scope, proto):
    return Response.str(BODY_STR_LONG, 200, HEADERS)


async def handle_404(scope, proto):
    return Response.str("not found", 404, HEADERS)


routes = {
    '/b': b_short,
    '/bb': b_long,
    '/s': s_short,
    '/ss': s_long
}


def app(scope, proto):
    handler = routes.get(scope.path, handle_404)
    return handler(scope, proto)


def granian(wrk, thr):
    from granian import Granian
    Granian("rsgi:app", workers=int(wrk), threads=int(thr), interface="rsgi").serve()


if __name__ == "__main__":
    granian(sys.argv[1], sys.argv[2])
