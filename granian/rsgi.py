from __future__ import annotations

import asyncio
import threading
import time
from collections.abc import AsyncIterator, Awaitable, Iterator
from enum import Enum
from typing import Any


try:
    import tonio.colored as tonio

    _TonioCancelled = tonio.exceptions.CancelledError
except Exception:
    tonio = None
    _TonioCancelled = None

from ._granian import (
    RSGIHeaders as Headers,
    RSGIHTTPProtocol as HTTPProtocol,
    RSGIHTTPReader as HTTPReader,
    RSGIHTTPWriter as HTTPWriter,
    RSGIProtocolClosed,
    RSGIProtocolError,
    RSGIWebsocketProtocol as WebsocketProtocol,
    RSGIWebsocketReader as WebsocketReader,
    RSGIWebsocketWriter as WebsocketWriter,
)
from ._interop import App as _App
from .log import log_request_builder, logger


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
    authority: str | None

    @property
    def headers(self) -> Headers: ...


class WebsocketMessageType(int, Enum):
    close = 0
    bytes = 1
    string = 2


class WebsocketMessage:
    kind: WebsocketMessageType
    data: bytes | str


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

    def on_websocket_wlog(self, proto: HTTPProtocol, scope: Scope):
        self._log(time.time(), time.perf_counter(), scope, 101)
        self.on_websocket(proto, scope)


class AsyncIORSGIApp(_App):
    __slots__ = ['_inner', '_log', '_loop', '_tasks']

    def __init__(self, app, loop, access_log_fmt=False):
        self._inner = app
        self._log = _build_access_logger(access_log_fmt)
        self._loop = loop
        self._tasks = set()

    async def _http(self, proto: HTTPProtocol, scope: Scope):
        try:
            await self._inner(scope, AsyncIORSGIHTTPProtocol(proto, self._loop))
        except Exception:
            logger.error('Application callable raised an exception', exc_info=True)
        finally:
            proto.close()

    async def _ws(self, proto: WebsocketProtocol, scope: Scope):
        try:
            await self._inner(scope, AsyncIORSGIWebsocketProtocol(proto, self._loop))
        except Exception:
            logger.error('Application callable raised an exception', exc_info=True)
        finally:
            proto.close()

    async def _http_wlog(self, proto: HTTPProtocol, scope: Scope):
        rt, mt = time.time(), time.perf_counter()
        lproto = _HTTPProtocolLoggingWrapper(proto)
        try:
            await self._inner(scope, AsyncIORSGIHTTPProtocol(lproto, self._loop))
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

    def write_bytes(self, data) -> Awaitable[None]:
        return self._write(self._inner.write_bytes, data)

    def write_str(self, data) -> Awaitable[None]:
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

    def read(self):
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


class _AsyncIOResume:
    __slots__ = ['_cb', '_fut', '_exc']

    def __init__(self, loop: asyncio.AbstractEventLoop, fut: asyncio.Future, exc: Any = RSGIProtocolClosed):
        self._cb = loop.call_soon_threadsafe
        self._fut = fut
        self._exc = exc

    def _ok(self, *res):
        self._cb(self._fut.set_result, res or None)

    def _err(self):
        self._cb(self._fut.set_exception, self._exc)


class AsyncIORSGIHTTPProtocol(_HTTPProtocolWrapper):
    __slots__ = ['_inner', '_loop']

    def __init__(self, inner: HTTPProtocol, loop: asyncio.AbstractEventLoop):
        self._inner = inner
        self._loop = loop

    async def watch(self) -> None:
        fut = self._loop.create_future()
        cb = _AsyncIOResume(self._loop, fut)
        if (cancel := self._inner.watch(cb._ok)) is None:
            return
        try:
            await fut
        except asyncio.CancelledError:
            cancel()
            raise

    async def read(self) -> bytes:
        fut = self._loop.create_future()
        cb = _AsyncIOResume(self._loop, fut)
        if (cancel := self._inner.read(cb._ok, cb._err)) is None:
            raise RSGIProtocolError
        try:
            res = await fut
        except asyncio.CancelledError:
            cancel()
            raise
        return res[0]

    def reader(self) -> AsyncIORSGIHTTPReader:
        return AsyncIORSGIHTTPReader(self._inner.reader(), self._loop)

    def writer(self, status: int, headers: list[tuple[str, str]]) -> AsyncIORSGIHTTPWriter:
        return AsyncIORSGIHTTPWriter(self._inner.writer(status, headers), self._loop)


class AsyncIORSGIHTTPReader:
    __slots__ = ['_inner', '_loop']

    def __init__(self, inner: HTTPReader, loop: asyncio.AbstractEventLoop):
        self._inner = inner
        self._loop = loop

    async def read(self) -> tuple[bytes, bool]:
        fut = self._loop.create_future()
        cb = _AsyncIOResume(self._loop, fut)

        cancel = self._inner.read(cb._ok)
        try:
            res = await fut
        except asyncio.CancelledError:
            cancel()
            raise

        return res

    async def __aiter__(self) -> AsyncIterator[bytes]:
        while True:
            data, eof = await self.read()
            yield data

            if eof:
                break


class AsyncIORSGIHTTPWriter:
    __slots__ = ['_inner', '_loop']

    def __init__(self, inner: HTTPWriter, loop: asyncio.AbstractEventLoop):
        self._inner = inner
        self._loop = loop

    async def _write(self, method, data) -> None:
        fut = self._loop.create_future()
        cb = _AsyncIOResume(self._loop, fut)

        cancel = method(data, cb._ok, cb._err)
        try:
            await fut
        except asyncio.CancelledError:
            cancel()
            raise

    def write_bytes(self, data) -> Awaitable[None]:
        return self._write(self._inner.write_bytes, data)

    def write_str(self, data) -> Awaitable[None]:
        return self._write(self._inner.write_str, data)


class AsyncIORSGIWebsocketProtocol(_WebsocketProtocolWrapper):
    __slots__ = ['_inner', '_loop']

    def __init__(self, inner: WebsocketProtocol, loop: asyncio.AbstractEventLoop):
        self._inner = inner
        self._loop = loop

    async def accept(self) -> tuple[AsyncIORSGIWebsocketReader, AsyncIORSGIWebsocketWriter]:
        fut = self._loop.create_future()
        cb = _AsyncIOResume(self._loop, fut, RSGIProtocolError)

        cancel = self._inner.accept(cb._ok, cb._err)
        try:
            res = await fut
        except asyncio.CancelledError:
            cancel()
            raise

        return AsyncIORSGIWebsocketReader(res[0], self._loop), AsyncIORSGIWebsocketWriter(res[1], self._loop)


class AsyncIORSGIWebsocketReader:
    __slots__ = ['_inner', '_loop']

    def __init__(self, inner: WebsocketReader, loop: asyncio.AbstractEventLoop):
        self._inner = inner
        self._loop = loop

    async def read(self) -> WebsocketMessage:
        fut = self._loop.create_future()
        cb = _AsyncIOResume(self._loop, fut)

        cancel = self._inner.read(cb._ok, cb._err)
        try:
            ret = await fut
        except asyncio.CancelledError:
            cancel()
            raise

        return ret[0]


class AsyncIORSGIWebsocketWriter:
    __slots__ = ['_inner', '_loop']

    def __init__(self, inner: WebsocketWriter, loop: asyncio.AbstractEventLoop):
        self._inner = inner
        self._loop = loop

    async def _write(self, method, data) -> None:
        fut = self._loop.create_future()
        cb = _AsyncIOResume(self._loop, fut)

        cancel = method(data, cb._ok, cb._err)
        try:
            await fut
        except asyncio.CancelledError:
            cancel()
            raise

    def write_bytes(self, data) -> Awaitable[None]:
        return self._write(self._inner.write_bytes, data)

    def write_str(self, data) -> Awaitable[None]:
        return self._write(self._inner.write_str, data)


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
