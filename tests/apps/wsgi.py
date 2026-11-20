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
                'raw_uri': environ['RAW_URI'],
                'content_length': environ.get('CONTENT_LENGTH'),
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
    return {
        '/info': info,
        # PATH_INFO for /info/%E6%B5%8B%2F: percent-decoded, then decoded as latin-1
        '/info/' + '测'.encode().decode('latin-1') + '/': info,
        '/echo': echo,
        '/iterbody': iterbody,
        '/err_app': err_app,
    }[environ['PATH_INFO']](environ, protocol)
