import os
import sys
import time
from functools import wraps
from typing import Any, Callable, Dict, List, Tuple

from .log import log_request_builder


class Response:
    __slots__ = ['status', 'headers']

    def __init__(self):
        self.status = 200
        self.headers = []

    def __call__(self, status: str, headers: List[Tuple[str, str]], exc_info: Any = None):
        self.status = int(status.split(' ', 1)[0])
        self.headers = headers


def _callback_wrapper(callback: Callable[..., Any], scope_opts: Dict[str, Any], access_log_fmt=None):
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

    def _runner(scope) -> Tuple[int, List[Tuple[str, str]], int, bytes]:
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

    def _logger(scope):
        t = time.time()
        try:
            rv = _runner(scope)
            access_log(t, scope, rv[0])
        except BaseException:
            access_log(t, scope, 500)
            raise
        return rv

    access_log = _build_access_logger(access_log_fmt)
    wrapper = _logger if access_log_fmt else _runner
    wraps(callback)(wrapper)
    return wrapper


def _build_access_logger(fmt):
    logger = log_request_builder(fmt)

    def access_log(t, scope, resp_code):
        logger(
            t,
            {
                'addr_remote': scope['REMOTE_ADDR'].split(':')[0],
                'protocol': scope['SERVER_PROTOCOL'],
                'path': scope['PATH_INFO'],
                'qs': scope['QUERY_STRING'],
                'method': scope['REQUEST_METHOD'],
                'scheme': scope['wsgi.url_scheme'],
            },
            resp_code,
        )

    return access_log
