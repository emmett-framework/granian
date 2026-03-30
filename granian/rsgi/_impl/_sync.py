from __future__ import annotations

import threading
import time
from collections.abc import Iterator

from ..._interop import App as _App
from ..types import HTTPProtocol, RSGIProtocolClosed, RSGIProtocolError, Scope, WebsocketMessage, WebsocketProtocol
from . import (
    HTTPReader,
    HTTPWriter,
    WebsocketReader,
    WebsocketWriter,
    _build_access_logger,
    _HTTPProtocolLoggingWrapper,
    _HTTPProtocolWrapper,
    _WebsocketProtocolWrapper,
)


class SyncRSGIApp(_App):
    __slots__ = ['_inner', '_log']

    def __init__(self, app, access_log_fmt=False):
        self._inner = app
        self._log = _build_access_logger(access_log_fmt)

    def on_request(self, proto: HTTPProtocol, scope: Scope):
        try:
            self._inner(scope, SyncRSGIHTTPProtocol(proto))
        finally:
            proto.close()

    def on_websocket(self, proto: WebsocketProtocol, scope: Scope):
        try:
            self._inner(scope, SyncRSGIWSProtocol(proto))
        finally:
            proto.close()

    def on_request_wlog(self, proto: HTTPProtocol, scope: Scope):
        rt, mt = time.time(), time.perf_counter()
        lproto = _HTTPProtocolLoggingWrapper(proto)
        try:
            self._inner(scope, SyncRSGIHTTPProtocol(lproto))
        finally:
            proto.close()
            self._log(rt, mt, scope, lproto._status)

    def on_websocket_wlog(self, proto: WebsocketProtocol, scope: Scope):
        self._log(time.time(), time.perf_counter(), scope, 101)
        self.on_websocket(proto, scope)


class SyncRSGIHTTPProtocol(_HTTPProtocolWrapper):
    __slots__ = []

    def watch(self):
        event = threading.Event()
        if self._inner.watch(event.set) is None:
            return
        event.wait()

    def read(self):
        event = threading.Event()
        ret = []

        def _ok(data):
            ret.append(data)
            event.set()

        def _err():
            ret.append(None)
            event.set()

        if self._inner.read(_ok, _err) is None:
            raise RSGIProtocolError
        event.wait()

        if (data := ret[0]) is None:
            raise RSGIProtocolClosed
        return data

    def reader(self) -> SyncRSGIHTTPReader:
        return SyncRSGIHTTPReader(self._inner.reader())

    def writer(self, status: int, headers: list[tuple[str, str]]) -> SyncRSGIHTTPWriter:
        return SyncRSGIHTTPWriter(self._inner.writer(status, headers))


class SyncRSGIHTTPReader:
    __slots__ = ['_inner']

    def __init__(self, inner: HTTPReader):
        self._inner = inner

    def read(self) -> tuple[bytes, bool]:
        event = threading.Event()
        ret = []

        def _ok(*vals):
            ret.append(vals)
            event.set()

        self._inner.read(_ok)
        event.wait()
        return ret[0]

    def __iter__(self) -> Iterator[bytes]:
        event = threading.Event()
        ret = []

        def _ok(*vals):
            ret.append(vals)
            event.set()

        while True:
            self._inner.read(_ok)
            event.wait()

            data, eof = ret.pop()
            yield data

            if eof:
                break
            event.clear()


class SyncRSGIHTTPWriter:
    __slots__ = ['_inner']

    def __init__(self, inner: HTTPWriter):
        self._inner = inner

    def _write(self, method, data) -> None:
        event = threading.Event()
        ret = []

        def _ok():
            ret.append(False)
            event.set()

        def _err():
            ret.append(True)
            event.set()

        method(data, _ok, _err)
        event.wait()

        is_err = ret[0]
        if is_err:
            raise RSGIProtocolClosed

    def write_bytes(self, data) -> None:
        return self._write(self._inner.write_bytes, data)

    def write_str(self, data) -> None:
        return self._write(self._inner.write_str, data)


class SyncRSGIWSProtocol(_WebsocketProtocolWrapper):
    __slots__ = []

    def accept(self):
        event = threading.Event()
        ret = []

        def _ok(*vals):
            ret.append(vals)
            event.set()

        def _err():
            ret.append(None)
            event.set()

        self._inner.accept(_ok, _err)
        event.wait()

        if (res := ret[0]) is None:
            raise RSGIProtocolError
        return SyncRSGIWebsocketReader(res[0]), SyncRSGIWebsocketWriter(res[1])


class SyncRSGIWebsocketReader:
    __slots__ = ['_inner']

    def __init__(self, inner: WebsocketReader):
        self._inner = inner

    def read(self) -> WebsocketMessage:
        event = threading.Event()
        ret = []

        def _ok(data):
            ret.append(data)
            event.set()

        def _err():
            ret.append(None)
            event.set()

        self._inner.read(_ok, _err)
        event.wait()

        if (res := ret[0]) is None:
            raise RSGIProtocolClosed
        return res


class SyncRSGIWebsocketWriter:
    __slots__ = ['_inner']

    def __init__(self, inner: WebsocketWriter):
        self._inner = inner

    def _write(self, method, data):
        event = threading.Event()
        ret = []

        def _ok():
            ret.append(False)
            event.set()

        def _err():
            ret.append(True)
            event.set()

        method(data, _ok, _err)
        event.wait()

        is_err = ret[0]
        if is_err:
            raise RSGIProtocolClosed

    def write_bytes(self, data):
        return self._write(self._inner.write_bytes, data)

    def write_str(self, data):
        return self._write(self._inner.write_str, data)
