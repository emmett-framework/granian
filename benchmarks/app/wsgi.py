HEADERS = [('content-type', 'text/plain; charset=utf-8')]

BODY_BYTES_SHORT = b"Test"
BODY_BYTES_LONG = b"Test" * 20_000
BODY_STR_SHORT = "Test"
BODY_STR_LONG = "Test" * 20_000


def b_short(environ, proto):
    proto('200 OK', HEADERS)
    return [BODY_BYTES_SHORT]


def b_long(environ, proto):
    proto('200 OK', HEADERS)
    return [BODY_BYTES_LONG]


def s_short(environ, proto):
    proto('200 OK', HEADERS)
    return [BODY_STR_SHORT.encode("utf8")]


def s_long(environ, proto):
    proto('200 OK', HEADERS)
    return [BODY_STR_LONG.encode("utf8")]


def handle_404(environ, proto):
    proto('404 NOT FOUND', HEADERS)
    return [b"not found"]


routes = {
    '/b': b_short,
    '/bb': b_long,
    '/s': s_short,
    '/ss': s_long
}


def app(environ, proto):
    handler = routes.get(environ["PATH_INFO"], handle_404)
    return handler(environ, proto)
