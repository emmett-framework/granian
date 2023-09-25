import json


def info(environ, protocol):
    protocol('200 OK', [('content-type', 'application/json')])
    return [
        json.dumps(
            {
                'scheme': environ['wsgi.url_scheme'],
                'method': environ['REQUEST_METHOD'],
                'path': environ['PATH_INFO'],
                'query_string': environ['QUERY_STRING'],
                'content_length': environ['CONTENT_LENGTH'],
                'headers': {k: v for k, v in environ.items() if k.startswith('HTTP_')},
            }
        ).encode('utf8')
    ]


def echo(environ, protocol):
    protocol('200 OK', [('content-type', 'text/plain; charset=utf-8')])
    return [environ['wsgi.input'].read()]


def iterbody(environ, protocol):
    def response():
        for _ in range(0, 3):
            yield b'test'

    protocol('200 OK', [('content-type', 'text/plain; charset=utf-8')])
    return response()


def err_app(environ, protocol):
    1 / 0


def app(environ, protocol):
    return {'/info': info, '/echo': echo, '/iterbody': iterbody, '/err_app': err_app}[environ['PATH_INFO']](
        environ, protocol
    )
