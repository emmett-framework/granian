from __future__ import annotations

import time
from collections.abc import AsyncIterator, Awaitable

from ..._interop import App as _App
from ...log import logger
from ..types import HTTPProtocol, Scope, WebsocketMessage, WebsocketProtocol
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


class AsyncIORSGIApp(_App):
    __slots__ = ['_inner', '_log', '_loop', '_tasks']

    def __init__(self, app, loop, access_log_fmt=False):
        self._inner = app
        self._log = _build_access_logger(access_log_fmt)
        self._loop = loop
        self._tasks = set()

    async def _http(self, proto: HTTPProtocol, scope: Scope):
        try:
            await self._inner(scope, AsyncIORSGIHTTPProtocol(proto))
        except Exception:
            logger.error('Application callable raised an exception', exc_info=True)
        finally:
            proto.close()

    async def _ws(self, proto: WebsocketProtocol, scope: Scope):
        try:
            await self._inner(scope, AsyncIORSGIWebsocketProtocol(proto))
        except Exception:
            logger.error('Application callable raised an exception', exc_info=True)
        finally:
            proto.close()

    async def _http_wlog(self, proto: HTTPProtocol, scope: Scope):
        rt, mt = time.time(), time.perf_counter()
        lproto = _HTTPProtocolLoggingWrapper(proto)
        try:
            await self._inner(scope, AsyncIORSGIHTTPProtocol(lproto))
        except Exception:
            logger.error('Application callable raised an exception', exc_info=True)
        finally:
            proto.close()
            self._log(rt, mt, scope, lproto._status)

    def _ws_wlog(self, proto: WebsocketProtocol, scope: Scope):
        self._log(time.time(), time.perf_counter(), scope, 101)
        return self._ws(proto, scope)

    def _crate_task(self, method, proto, scope):
        task = self._loop.create_task(method(proto, scope))
        self._tasks.add(task)
        task.add_done_callback(self._tasks.discard)

    def on_request(self, proto: HTTPProtocol, scope: Scope):
        self._loop.call_soon_threadsafe(self._crate_task, self._http, proto, scope)

    def on_websocket(self, proto: WebsocketProtocol, scope: Scope):
        self._loop.call_soon_threadsafe(self._crate_task, self._ws, proto, scope)

    def on_request_wlog(self, proto: HTTPProtocol, scope: Scope):
        self._loop.call_soon_threadsafe(self._crate_task, self._http_wlog, proto, scope)

    def on_websocket_wlog(self, proto: WebsocketProtocol, scope: Scope):
        self._loop.call_soon_threadsafe(self._crate_task, self._ws_wlog, proto, scope)


class AsyncIORSGIHTTPProtocol(_HTTPProtocolWrapper):
    __slots__ = ['_inner']

    def __init__(self, inner: HTTPProtocol):
        self._inner = inner

    def watch(self) -> Awaitable[None]:
        return self._inner._watch_asyncio()

    def read(self) -> Awaitable[bytes]:
        return self._inner._read_asyncio()

    def reader(self) -> AsyncIORSGIHTTPReader:
        return AsyncIORSGIHTTPReader(self._inner.reader())

    def writer(self, status: int, headers: list[tuple[str, str]]) -> AsyncIORSGIHTTPWriter:
        return AsyncIORSGIHTTPWriter(self._inner.writer(status, headers))


class AsyncIORSGIHTTPReader:
    __slots__ = ['_inner']

    def __init__(self, inner: HTTPReader):
        self._inner = inner

    def read(self) -> Awaitable[tuple[bytes, bool]]:
        return self._inner._read_asyncio()

    async def __aiter__(self) -> AsyncIterator[bytes]:
        while True:
            data, eof = await self._inner._read_asyncio()
            yield data

            if eof:
                break


class AsyncIORSGIHTTPWriter:
    __slots__ = ['_inner', '_loop']

    def __init__(self, inner: HTTPWriter):
        self._inner = inner

    def write_bytes(self, data) -> Awaitable[None]:
        return self._inner._write_asyncio_bytes(data)

    def write_str(self, data) -> Awaitable[None]:
        return self._inner._write_asyncio_str(data)


class AsyncIORSGIWebsocketProtocol(_WebsocketProtocolWrapper):
    __slots__ = ['_inner']

    def __init__(self, inner: WebsocketProtocol):
        self._inner = inner

    async def accept(self) -> tuple[AsyncIORSGIWebsocketReader, AsyncIORSGIWebsocketWriter]:
        reader, writer = self._inner._accept_asyncio()
        return AsyncIORSGIWebsocketReader(reader), AsyncIORSGIWebsocketWriter(writer)


class AsyncIORSGIWebsocketReader:
    __slots__ = ['_inner']

    def __init__(self, inner: WebsocketReader):
        self._inner = inner

    def read(self) -> Awaitable[WebsocketMessage]:
        return self._inner._read_asyncio()


class AsyncIORSGIWebsocketWriter:
    __slots__ = ['_inner']

    def __init__(self, inner: WebsocketWriter):
        self._inner = inner

    def write_bytes(self, data) -> Awaitable[None]:
        return self._inner._write_asyncio_bytes(data)

    def write_str(self, data) -> Awaitable[None]:
        return self._inner._write_asyncio_str(data)
