import sys

from granian.rsgi import Response

HEADERS = {'content-type': 'text/plain; charset=utf-8'}

BODY_BYTES_SHORT = b"Test"
BODY_BYTES_LONG = b"Test" * 20_000
BODY_STR_SHORT = "Test"
BODY_STR_LONG = "Test" * 20_000


async def b_short(scope, receive):
    return Response(
        1, 200, HEADERS, BODY_BYTES_SHORT, None, None
    )


async def b_long(scope, receive):
    return Response(
        1, 200, HEADERS, BODY_BYTES_LONG, None, None
    )


async def s_short(scope, receive):
    return Response(
        2, 200, HEADERS, None, BODY_STR_SHORT, None
    )


async def s_long(scope, receive):
    return Response(
        2, 200, HEADERS, None, BODY_STR_LONG, None
    )


async def handle_404(scope, receive):
    return Response(
        2, 200, HEADERS, None, "not found", None
    )


routes = {
    '/b': b_short,
    '/bb': b_long,
    '/s': s_short,
    '/ss': s_long
}


def app(scope, receive):
    handler = routes.get(scope.path, handle_404)
    return handler(scope, receive)


def granian(wrk, thr):
    from granian import Granian
    Granian("rsgi:app", workers=int(wrk), threads=int(thr), interface="rsgi").serve()


if __name__ == "__main__":
    granian(sys.argv[1], sys.argv[2])
