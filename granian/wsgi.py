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

    def __call__(self, status: str, headers: List[Tuple[str, str]], exc_info: Any = None):
        self.status = int(status.split(' ', 1)[0])
        self.headers = headers


def _callback_wrapper(callback, scope_opts):
    basic_env = dict(os.environ)
    basic_env.update(
        {
            'GATEWAY_INTERFACE': 'CGI/1.1',
            'SCRIPT_NAME': scope_opts.get('url_path_prefix') or '',
            'SERVER_SOFTWARE': 'Granian',
            'wsgi.errors': sys.stderr,
            'wsgi.input_terminated': True,
            'wsgi.input': None,
            'wsgi.multiprocess': False,
            'wsgi.multithread': False,
            'wsgi.run_once': False,
            'wsgi.version': (1, 0),
        }
    )

    @wraps(callback)
    def wrapper(scope: Scope) -> Tuple[int, List[Tuple[str, str]], bytes]:
        resp = Response()
        rv = callback(scope.to_environ(dict(basic_env)), resp)

        if isinstance(rv, list):
            resp_type = 0
            rv = b''.join(rv)
        else:
            resp_type = 1
            rv = iter(rv)

        return (resp.status, resp.headers, resp_type, rv)

    return wrapper
