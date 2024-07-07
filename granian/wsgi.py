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


class ResponseIterWrap:
    __slots__ = ['inner', '__next__']

    def __init__(self, inner):
        self.inner = inner
        self.__next__ = iter(inner).__next__

    def close(self):
        self.inner.close()


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

    def _runner(proto, scope):
        resp = Response()
        scope.update(basic_env)
        rv = callback(scope, resp)

        if isinstance(rv, list):
            proto.response_bytes(resp.status, resp.headers, b''.join(rv))
        else:
            proto.response_iter(resp.status, resp.headers, ResponseIterWrap(rv))

        return resp.status

    def _logger(proto, scope):
        t = time.time()
        try:
            status = _runner(proto, scope)
            access_log(t, scope, status)
        except BaseException:
            access_log(t, scope, 500)
            raise
        return status

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
