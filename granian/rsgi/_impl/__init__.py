from ..._granian import (
    RSGIHTTPReader as HTTPReader,  # noqa: F401
    RSGIHTTPWriter as HTTPWriter,  # noqa: F401
    RSGIWebsocketReader as WebsocketReader,  # noqa: F401
    RSGIWebsocketWriter as WebsocketWriter,  # noqa: F401
)
from ...log import log_request_builder
from ..types import HTTPProtocol, Scope, WebsocketProtocol


class _HTTPProtocolWrapper:
    __slots__ = ['_inner']

    def __init__(self, inner: HTTPProtocol):
        self._inner = inner

    @property
    def write(self):
        return self._inner.write

    @property
    def write_bytes(self):
        return self._inner.write_bytes

    @property
    def write_str(self):
        return self._inner.write_str

    @property
    def write_file(self):
        return self._inner.write_file

    @property
    def write_file_range(self):
        return self._inner.write_file_range

    @property
    def writer(self):
        return self._inner.writer


class _HTTPProtocolLoggingWrapper:
    __slots__ = ['_inner', '_status']

    def __init__(self, inner: HTTPProtocol):
        self._inner = inner
        self._status = 500

    @property
    def close(self):
        return self._inner.close

    @property
    def read(self):
        return self._inner.read

    @property
    def reader(self):
        return self._inner.reader

    @property
    def watch(self):
        return self._inner.watch

    def write(self, status, headers):
        self._status = status
        return self._inner.write(status, headers)

    def write_bytes(self, status, headers, body):
        self._status = status
        return self._inner.write_bytes(status, headers, body)

    def write_str(self, status, headers, body):
        self._status = status
        return self._inner.write_str(status, headers, body)

    def write_file(self, status, headers, file):
        self._status = status
        return self._inner.write_file(status, headers, file)

    def write_file_range(self, status, headers, file, start, end):
        self._status = status
        return self._inner.write_file_range(status, headers, file, start, end)

    def writer(self, status, headers):
        self._status = status
        return self._inner.writer(status, headers)


class _WebsocketProtocolWrapper:
    __slots__ = ['_inner']

    def __init__(self, inner: WebsocketProtocol):
        self._inner = inner

    @property
    def close(self):
        return self._inner.close


def _callbacks_from_target(target):
    callback = getattr(target, '__rsgi__') if hasattr(target, '__rsgi__') else target
    callback_init = (
        getattr(target, '__rsgi_init__') if hasattr(target, '__rsgi_init__') else lambda *args, **kwargs: None
    )
    callback_del = getattr(target, '__rsgi_del__') if hasattr(target, '__rsgi_del__') else lambda *args, **kwargs: None
    return callback, callback_init, callback_del


def _build_access_logger(fmt):
    logger = log_request_builder(fmt)

    def _log_dict(scope: Scope):
        return {
            'addr_remote': scope.client.rsplit(':', 1)[0],
            'protocol': 'HTTP/' + scope.http_version,
            'path': scope.path,
            'qs': scope.query_string,
            'method': scope.method,
            'scheme': scope.scheme,
        }

    def _access_log(rt, mt, scope, resp_code):
        logger(rt, mt, _log_dict(scope), resp_code)

    def _access_log_with_headers(rt, mt, scope: Scope, resp_code):
        data = _log_dict(scope)
        data['headers'] = scope.headers.get
        logger(rt, mt, data, resp_code)

    return _access_log_with_headers if logger.parse_headers else _access_log
