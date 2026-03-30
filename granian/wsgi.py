import os
import sys
import time
from typing import Any

from ._interop import App as _App
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
        self.inner.close()


def _build_access_logger(fmt):
    logger = log_request_builder(fmt)

    def _log_dict(scope):
        return {
            'addr_remote': scope['REMOTE_ADDR'].rsplit(':', 1)[0],
            'protocol': scope['SERVER_PROTOCOL'],
            'path': scope['PATH_INFO'],
            'qs': scope['QUERY_STRING'],
            'method': scope['REQUEST_METHOD'],
            'scheme': scope['wsgi.url_scheme'],
        }

    def _access_log(rt, mt, scope, resp_code):
        logger(rt, mt, _log_dict(scope), resp_code)

    def _access_log_with_headers(rt, mt, scope, resp_code):
        data = _log_dict(scope)
        data['headers'] = lambda key: scope.get('HTTP_' + key.upper().replace('-', '_'))
        logger(rt, mt, data, resp_code)

    return _access_log_with_headers if logger.parse_headers else _access_log


class WSGIApp(_App):
    __slots__ = ['_inner', '_log', '_environ', '_prefix_len']

    def __init__(self, app, scope_opts=None, access_log_fmt=None):
        self._inner = app
        self._log = _build_access_logger(access_log_fmt)
        self._environ = self._build_basic_environ(scope_opts or {})
        self._prefix_len = len(self._environ['SCRIPT_NAME'])

    @staticmethod
    def _build_basic_environ(scope_opts):
        basic_env: dict[str, Any] = dict(os.environ)
        basic_env.update(
            {
                'GATEWAY_INTERFACE': 'CGI/1.1',
                'SCRIPT_NAME': scope_opts.get('url_path_prefix') or '',
                'SERVER_SOFTWARE': 'Granian',
                'wsgi.errors': sys.stderr,
                #: this is not in PEP333, but you know, werkzeug..
                'wsgi.input_terminated': True,
                'wsgi.multiprocess': False,
                'wsgi.multithread': True,
                'wsgi.run_once': False,
                'wsgi.version': (1, 0),
            }
        )
        return basic_env

    def on_request(self, proto, scope):
        resp = Response()
        environ = self._environ | scope
        if self._environ['SCRIPT_NAME']:
            environ['PATH_INFO'] = scope['PATH_INFO'][self._prefix_len :] or '/'

        try:
            rv = self._inner(environ, resp)
            if isinstance(rv, list):
                proto.response_bytes(resp.status, resp.headers, b''.join(rv))
            else:
                proto.response_iter(resp.status, resp.headers, ResponseIterWrap(rv))
        finally:
            proto._close()

    def on_request_wlog(self, proto, scope):
        rt, mt = time.time(), time.perf_counter()
        resp = Response()
        environ = self._environ | scope
        if self._environ['SCRIPT_NAME']:
            environ['PATH_INFO'] = scope['PATH_INFO'][self._prefix_len :] or '/'

        try:
            rv = self._inner(environ, resp)
            if isinstance(rv, list):
                proto.response_bytes(resp.status, resp.headers, b''.join(rv))
            else:
                proto.response_iter(resp.status, resp.headers, ResponseIterWrap(rv))
            self._log(rt, mt, scope, resp.status)
        except BaseException:
            self._log(rt, mt, scope, 500)
            raise
        finally:
            proto._close()

    def on_websocket(self, proto, scope):
        raise NotImplementedError

    def on_websocket_wlog(self, proto, scope):
        raise NotImplementedError
