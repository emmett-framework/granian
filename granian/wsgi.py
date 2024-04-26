import os
import sys
from functools import wraps
from typing import Any, Callable, Dict, List, Tuple


class Response:
    __slots__ = ['status', 'headers']

    def __init__(self):
        self.status = 200
        self.headers = []

    def __call__(self, status: str, headers: List[Tuple[str, str]], exc_info: Any = None):
        self.status = int(status.split(' ', 1)[0])
        self.headers = headers


def _callback_wrapper(callback: Callable[..., Any], scope_opts: Dict[str, Any]):
    basic_env: Dict[str, Any] = dict(os.environ)
    basic_env.update(
        {
            'GATEWAY_INTERFACE': 'CGI/1.1',
            'SCRIPT_NAME': scope_opts.get('url_path_prefix') or '',
            'SERVER_SOFTWARE': 'Granian',
            'wsgi.errors': sys.stderr,
            'wsgi.multiprocess': False,
            'wsgi.multithread': True,
            'wsgi.run_once': False,
            'wsgi.version': (1, 0),
        }
    )

    @wraps(callback)
    def wrapper(scope) -> Tuple[int, List[Tuple[str, str]], int, bytes]:
        resp = Response()
        scope.update(basic_env)
        rv = callback(scope, resp)

        if isinstance(rv, list):
            resp_type = 0
            rv = b''.join(rv)
        else:
            resp_type = 1
            rv = iter(rv)

        return (resp.status, resp.headers, resp_type, rv)

    return wrapper
