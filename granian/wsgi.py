import os
import sys
import time
from collections.abc import Callable
from functools import wraps
from typing import Any

from .log import log_request_builder


class Response:
    __slots__ = ['status', 'headers']

    def __init__(self):
        self.status = 200
        self.headers = []

    def __call__(self, status: str, headers: list[tuple[str, str]], exc_info: Any = None):
        self.status = int(status.split(' ', 1)[0])
        self.headers = headers


class ResponseIterWrap:
    __slots__ = ['inner', '__next__']

    def __init__(self, inner):
        self.inner = inner
        self.__next__ = iter(inner).__next__

    def close(self):
        if hasattr(self.inner, 'close'):
            self.inner.close()


class _LoggingProto:
    __slots__ = ['inner', 'resp_headers']

    def __init__(self, inner):
        self.inner = inner
        self.resp_headers = ()

    def response_bytes(self, status, headers, body):
        self.resp_headers = headers
        return self.inner.response_bytes(status, headers, body)

    def response_iter(self, status, headers, body):
        self.resp_headers = headers
        return self.inner.response_iter(status, headers, body)


def _callback_wrapper(callback: Callable[..., Any], scope_opts: dict[str, Any], access_log_fmt=None):
    basic_env: dict[str, Any] = dict(os.environ)
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
        if scope['SCRIPT_NAME']:
            scope['PATH_INFO'] = scope['PATH_INFO'][len(scope['SCRIPT_NAME']) :] or '/'

        rv = callback(scope, resp)

        if isinstance(rv, list):
            proto.response_bytes(resp.status, resp.headers, b''.join(rv))
        else:
            proto.response_iter(resp.status, resp.headers, ResponseIterWrap(rv))

        return resp.status

    def _logger(proto, scope):
        rt, mt = time.time(), time.perf_counter()
        try:
            status = _runner(proto, scope)
            access_log(rt, mt, scope, status)
        except BaseException:
            access_log(rt, mt, scope, 500)
            raise
        return status

    def _logger_with_resp_headers(proto, scope):
        rt, mt = time.time(), time.perf_counter()
        lproto = _LoggingProto(proto)
        try:
            status = _runner(lproto, scope)
            access_log(rt, mt, scope, status, lproto.resp_headers)
        except BaseException:
            access_log(rt, mt, scope, 500, lproto.resp_headers)
            raise
        return status

    access_log, _needs_resp_headers = _build_access_logger(access_log_fmt)
    if access_log_fmt:
        wrapper = _logger_with_resp_headers if _needs_resp_headers else _logger
    else:
        wrapper = _runner
    wraps(callback)(wrapper)
    return wrapper


def _build_access_logger(fmt):
    logger = log_request_builder(fmt)
    _needs_resp_headers = logger.needs_resp_headers

    def access_log(rt, mt, scope, resp_code, resp_headers=()):
        def get_header(name):
            return scope.get('HTTP_' + name.upper().replace('-', '_'))

        req = {
            'addr_remote': scope['REMOTE_ADDR'].rsplit(':', 1)[0],
            'protocol': scope['SERVER_PROTOCOL'],
            'path': scope['PATH_INFO'],
            'qs': scope['QUERY_STRING'],
            'method': scope['REQUEST_METHOD'],
            'scheme': scope['wsgi.url_scheme'],
            'user_agent': scope.get('HTTP_USER_AGENT', '-'),
            'get_header': get_header,
        }
        if _needs_resp_headers:
            # WSGI response headers are [(str, str)] e.g. [('Content-Type', 'application/json')]
            req['get_response_header'] = {hname.lower(): hval for hname, hval in resp_headers}.get
        logger(rt, mt, req, resp_code)

    return access_log, _needs_resp_headers
