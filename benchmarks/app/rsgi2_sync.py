import pathlib
import time


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

    def route(scope, proto):
        proto.write_bytes(200, HEADERS, body)

    return route


def s_builder(size):
    body = BODY_STR[size]

    def route(scope, proto):
        proto.write_str(200, HEADERS, body)

    return route


def echo(scope, proto):
    proto.write_bytes(200, HEADERS, proto.read())


def echo_iter(scope, proto):
    trx = proto.writer(200, HEADERS)
    print(trx)
    for chunk in proto.reader():
        trx.write_bytes(chunk)


def file(scope, proto):
    proto.write_file(200, HEADERS_MEDIA, MEDIA_PATH)


def io_builder(wait):
    wait = wait / 1000

    def io(scope, proto):
        time.sleep(wait)
        proto.write_bytes(200, HEADERS, BODY_BYTES[10])

    return io


def handle_404(scope, proto):
    proto.write_str(404, HEADERS, 'not found')


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
