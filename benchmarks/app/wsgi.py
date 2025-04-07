import time


HEADERS = [('content-type', 'text/plain; charset=utf-8')]

BODY_BYTES = {
    10: b'x' * 10,
    1000: b'x' * 1024,
    10_000: b'x' * 1024 * 10,
    100_000: b'x' * 1024 * 100,
}


def b_builder(size):
    body = BODY_BYTES[size]

    def route(environ, proto):
        proto('200 OK', HEADERS)
        return [body]

    return route


def echo(environ, proto):
    proto('200 OK', HEADERS)
    return [environ['wsgi.input'].read()]


def echo_iter(environ, proto):
    proto('200 OK', HEADERS)
    while True:
        data = environ['wsgi.input'].read(1024 * 16)
        if not data:
            break
        yield data


def io_builder(wait):
    wait = wait / 1000

    def io(environ, proto):
        proto('200 OK', HEADERS)
        time.sleep(wait)
        return [BODY_BYTES[10]]

    return io


def handle_404(environ, proto):
    proto('404 NOT FOUND', HEADERS)
    return [b'not found']


routes = {
    '/b10': b_builder(10),
    '/b1k': b_builder(1000),
    '/b10k': b_builder(10_000),
    '/b100k': b_builder(100_000),
    '/echo': echo,
    '/echoi': echo_iter,
    '/io10': io_builder(10),
    '/io100': io_builder(100),
}


def app(environ, proto):
    handler = routes.get(environ['PATH_INFO'], handle_404)
    return handler(environ, proto)
