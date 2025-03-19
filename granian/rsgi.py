import time
from enum import Enum
from functools import wraps
from typing import Optional, Union

from ._granian import (
    RSGIHeaders as Headers,
    RSGIHTTPProtocol as HTTPProtocol,  # noqa: F401
    RSGIProtocolClosed as ProtocolClosed,  # noqa: F401
    RSGIProtocolError as ProtocolError,  # noqa: F401
    RSGIWebsocketProtocol as WebsocketProtocol,  # noqa: F401
)
from .log import log_request_builder


class Scope:
    proto: str
    http_version: str
    rsgi_version: str
    server: str
    client: str
    scheme: str
    method: str
    path: str
    query_string: str
    authority: Optional[str]

    @property
    def headers(self) -> Headers: ...


class WebsocketMessageType(int, Enum):
    close = 0
    bytes = 1
    string = 2


class WebsocketMessage:
    kind: WebsocketMessageType
    data: Union[bytes, str]


class _LoggingProto:
    __slots__ = ['inner', 'status']

    def __init__(self, inner):
        self.inner = inner
        self.status = 500

    def __call__(self):
        return self.inner()

    def __aiter__(self):
        return self.inner.__aiter__()

    def client_disconnect(self):
        return self.inner.client_disconnect()

    def response_empty(self, status, headers):
        self.status = status
        return self.inner.response_empty(status, headers)

    def response_str(self, status, headers, body):
        self.status = status
        return self.inner.response_str(status, headers, body)

    def response_bytes(self, status, headers, body):
        self.status = status
        return self.inner.response_bytes(status, headers, body)

    def response_file(self, status, headers, file):
        self.status = status
        return self.inner.response_file(status, headers, file)

    def response_stream(self, status, headers):
        self.status = status
        return self.inner.response_stream(status, headers)


def _callback_wrapper(callback, access_log_fmt=False):
    async def _http_logger(scope, proto):
        t = time.time()
        try:
            rv = await callback(scope, proto)
        finally:
            access_log(t, scope, proto.status)
        return rv

    def _ws_logger(scope, proto):
        access_log(time.time(), scope, 101)
        return callback(scope, proto)

    def _logger(scope, proto):
        if scope.proto == 'http':
            return _http_logger(scope, _LoggingProto(proto))
        return _ws_logger(scope, proto)

    access_log = _build_access_logger(access_log_fmt)
    wrapper = callback
    if access_log_fmt:
        wrapper = _logger
        wraps(callback)(wrapper)
    return wrapper


def _build_access_logger(fmt):
    logger = log_request_builder(fmt)

    def access_log(t, scope, resp_code):
        logger(
            t,
            {
                'addr_remote': scope.client.rsplit(':', 1)[0],
                'protocol': 'HTTP/' + scope.http_version,
                'path': scope.path,
                'qs': scope.query_string,
                'method': scope.method,
                'scheme': scope.scheme,
            },
            resp_code,
        )

    return access_log
