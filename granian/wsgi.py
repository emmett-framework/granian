import os

from functools import wraps
from typing import List, Tuple

from ._granian import WSGIScope as Scope


class Response:
    __slots__ = ['status', 'headers']

    def __init__(self):
        self.status = 200
        self.headers = []


def _callback_wrapper(callback):
    basic_env = dict(os.environ)
    basic_env.update({
        'GATEWAY_INTERFACE': 'CGI/1.1',
        'SCRIPT_NAME': '',
        'SERVER_PROTOCOL': 'HTTP/1.1',
        'SERVER_SOFTWARE': 'Granian',
        'wsgi.errors': None,
        'wsgi.file_wrapper': None,
        'wsgi.input_terminated': True,
        'wsgi.input': None,
        'wsgi.multiprocess': False,
        'wsgi.multithread': False,
        'wsgi.run_once': False,
        'wsgi.version': (1, 0)
    })

    @wraps(callback)
    def wrapper(scope: Scope) -> Tuple[int, List[Tuple[str, str]], bytes]:
        addr_server = scope.server.split(":")
        environ = {
            **basic_env,
            **scope.headers,
            'SERVER_NAME': addr_server[0],
            'SERVER_PORT': str(addr_server[1]),
            'REQUEST_METHOD': scope.method,
            'PATH_INFO': scope.path,
            'QUERY_STRING': scope.query_string,
            'REMOTE_ADDR': scope.client,
            'wsgi.url_scheme': scope.scheme,
            'wsgi.input': scope.body
        }
        resp = Response()

        def start_response(status: str, headers: List[Tuple[str, str]]):
            resp.status = int(status.split(' ', 1)[0])
            resp.headers = headers

        rv = callback(environ, start_response)
        return (resp.status, resp.headers, b"".join(rv))

    return wrapper
