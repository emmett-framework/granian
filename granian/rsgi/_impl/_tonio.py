from __future__ import annotations

import time
from collections.abc import AsyncIterator, Awaitable

from ..._interop import App as _App
from ...log import logger
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


try:
    import tonio.colored as tonio

    _TonioCancelled = tonio.exceptions.CancelledError
except Exception:
    tonio = None
    _TonioCancelled = None


class TonioRSGIApp(_App):
    __slots__ = ['_inner', '_log']

    def __init__(self, app, access_log_fmt=False):
        self._inner = app
        self._log = _build_access_logger(access_log_fmt)

    async def _http(self, proto: HTTPProtocol, scope: Scope):
        try:
            await self._inner(scope, TonioRSGIHTTPProtocol(proto))
        except Exception:
            logger.error('Application callable raised an exception', exc_info=True)
        finally:
            proto.close()

    async def _ws(self, proto: WebsocketProtocol, scope: Scope):
        try:
            await self._inner(scope, TonioRSGIWebsocketProtocol(proto))
        except Exception:
            logger.error('Application callable raised an exception', exc_info=True)
        finally:
            proto.close()

    async def _http_wlog(self, proto: HTTPProtocol, scope: Scope):
        rt, mt = time.time(), time.perf_counter()
        lproto = _HTTPProtocolLoggingWrapper(proto)
        try:
            await self._inner(scope, TonioRSGIHTTPProtocol(lproto))
        except Exception:
            logger.error('Application callable raised an exception', exc_info=True)
        finally:
            proto.close()
            # TODO: compute actual time now
            tonio.spawn_blocking(self._log, rt, mt, scope, lproto._status)

    def _ws_wlog(self, proto: WebsocketProtocol, scope: Scope):
        tonio.spawn_blocking(self._log, time.time(), time.perf_counter(), scope, 101)
        return self._ws(proto, scope)

    def on_request(self, proto: HTTPProtocol, scope: Scope):
        tonio.spawn.without_tracking(self._http(proto, scope))

    def on_websocket(self, proto: WebsocketProtocol, scope: Scope):
        tonio.spawn.without_tracking(self._ws(proto, scope))

    def on_request_wlog(self, proto: HTTPProtocol, scope: Scope):
        tonio.spawn.without_tracking(self._http_wlog(proto, scope))

    def on_websocket_wlog(self, proto: WebsocketProtocol, scope: Scope):
        tonio.spawn.without_tracking(self._ws_wlog(proto, scope))


class TonioRSGIHTTPProtocol(_HTTPProtocolWrapper):
    __slots__ = []

    async def watch(self) -> None:
        event = tonio.Event()
        if (cancel := self._inner.watch(event.set)) is None:
            return
        try:
            await event.wait()
        except _TonioCancelled:
            cancel()
            raise

    async def read(self) -> bytes:
        event = tonio.Event()
        ret = tonio.Result()

        def _ok(data):
            ret.store(data)
            event.set()

        def _err():
            event.set()

        if (cancel := self._inner.read(_ok, _err)) is None:
            raise RSGIProtocolError
        try:
            await event.wait()
        except _TonioCancelled:
            cancel()
            raise

        if (res := ret.fetch()) is None:
            raise RSGIProtocolClosed
        return res

    def reader(self) -> TonioRSGIHTTPReader:
        return TonioRSGIHTTPReader(self._inner.reader())

    def writer(self, status: int, headers: list[tuple[str, str]]) -> TonioRSGIHTTPWriter:
        return TonioRSGIHTTPWriter(self._inner.writer(status, headers))


class TonioRSGIHTTPReader:
    __slots__ = ['_inner']

    def __init__(self, inner: HTTPReader):
        self._inner = inner

    async def read(self) -> tuple[bytes, bool]:
        event = tonio.Event()
        ret = tonio.Result()

        def _ok(*vals):
            ret.store(vals)
            event.set()

        cancel = self._inner.read(_ok)
        try:
            await event.wait()
        except _TonioCancelled:
            cancel()
            raise

        return ret.fetch()

    async def __aiter__(self) -> AsyncIterator[bytes]:
        event = tonio.Event()
        ret = []

        def _ok(*vals):
            ret.append(vals)
            event.set()

        while True:
            cancel = self._inner.read(_ok)
            try:
                await event.wait()
            except _TonioCancelled:
                cancel()
                raise

            data, eof = ret.pop()
            yield data

            if eof:
                break
            event.clear()


class TonioRSGIHTTPWriter:
    __slots__ = ['_inner']

    def __init__(self, inner: HTTPWriter):
        self._inner = inner

    async def _write(self, method, data) -> None:
        event = tonio.Event()
        ret = tonio.Result()

        def _ok():
            event.set()

        def _err():
            ret.store(True)
            event.set()

        cancel = method(data, _ok, _err)
        try:
            await event.wait()
        except _TonioCancelled:
            cancel()
            raise

        if ret.fetch():
            raise RSGIProtocolClosed

    def write_bytes(self, data) -> Awaitable[None]:
        return self._write(self._inner.write_bytes, data)

    def write_str(self, data) -> Awaitable[None]:
        return self._write(self._inner.write_str, data)


class TonioRSGIWebsocketProtocol(_WebsocketProtocolWrapper):
    __slots__ = []

    async def accept(self) -> tuple[TonioRSGIWebsocketReader, TonioRSGIWebsocketWriter]:
        event = tonio.Event()
        ret = tonio.Result()

        def _ok(*vals):
            ret.store(vals)
            event.set()

        def _err():
            event.set()

        cancel = self._inner.accept(_ok, _err)
        try:
            await event.wait()
        except _TonioCancelled:
            cancel()
            raise

        if (res := ret.fetch()) is None:
            raise RSGIProtocolError
        return TonioRSGIWebsocketReader(res[0]), TonioRSGIWebsocketWriter(res[1])


class TonioRSGIWebsocketReader:
    __slots__ = ['_inner']

    def __init__(self, inner: WebsocketReader):
        self._inner = inner

    async def read(self) -> WebsocketMessage:
        event = tonio.Event()
        ret = tonio.Result()

        def _ok(data):
            ret.store(data)
            event.set()

        def _err():
            event.set()

        cancel = self._inner.read(_ok, _err)
        try:
            await event.wait()
        except _TonioCancelled:
            cancel()
            raise

        if (res := ret.fetch()) is None:
            raise RSGIProtocolClosed
        return res


class TonioRSGIWebsocketWriter:
    __slots__ = ['_inner']

    def __init__(self, inner: WebsocketWriter):
        self._inner = inner

    async def _write(self, method, data) -> None:
        event = tonio.Event()
        ret = tonio.Result()

        def _ok():
            event.set()

        def _err():
            ret.store(True)
            event.set()

        cancel = method(data, _ok, _err)
        try:
            await event.wait()
        except _TonioCancelled:
            cancel()
            raise

        if ret.fetch():
            raise RSGIProtocolClosed

    def write_bytes(self, data) -> Awaitable[None]:
        return self._write(self._inner.write_bytes, data)

    def write_str(self, data) -> Awaitable[None]:
        return self._write(self._inner.write_str, data)
