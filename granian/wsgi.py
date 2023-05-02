import os
import sys

from functools import wraps
from typing import Any, List, Tuple

from ._granian import WSGIScope as Scope


class Response:
    __slots__ = ['status', 'headers']

    def __init__(self):
        self.status = 200
        self.headers = []

    def __call__(
        self,
        status: str,
        headers: List[Tuple[str, str]],
        exc_info: Any = None
    ):
        self.status = int(status.split(' ', 1)[0])
        self.headers = headers


def _callback_wrapper(callback, scope_opts):
    basic_env = dict(os.environ)
    basic_env.update({
        'GATEWAY_INTERFACE': 'CGI/1.1',
        'SCRIPT_NAME': scope_opts.get('url_path_prefix') or '',
        'SERVER_PROTOCOL': 'HTTP/1.1',
        'SERVER_SOFTWARE': 'Granian',
        'wsgi.errors': sys.stderr,
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
            'wsgi.input': scope.input()
        }
        if 'HTTP_CONTENT_TYPE' in environ:
            environ['CONTENT_TYPE'] = environ.pop('HTTP_CONTENT_TYPE')
        if 'HTTP_CONTENT_LENGTH' in environ:
            environ['CONTENT_LENGTH'] = environ.pop('HTTP_CONTENT_LENGTH')

        resp = Response()
        rv = callback(environ, resp)

        try:
            body = b"".join(rv)
        finally:
            if hasattr(rv, "close"):
                rv.close()

        return (resp.status, resp.headers, body)

    return wrapper
