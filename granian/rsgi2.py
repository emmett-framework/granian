import abc
import asyncio
import threading


try:
    import tonio.colored as tonio
    from tonio._tonio import ResultHolder as TonioResult
    from tonio.exceptions import CancelledError as _TonioCancelled
except Exception:
    tonio = None
    _TonioCancelled = None
    TonioResult = None

from ._granian import RSGIApp as _RSGIApp
from .log import logger


class RSGIApp(abc.ABC):
    def rsgi(self):
        return _RSGIApp(self)

    @abc.abstractmethod
    def on_request(self, proto, scope): ...

    # @abc.abstractmethod
    # def on_websocket(self, proto, scope):
    #     ...


class _RSGIProtoWrapper:
    def __init__(self, proto):
        self._proto = proto

    @property
    def write(self):
        return self._proto.write

    @property
    def write_bytes(self):
        return self._proto.write_bytes

    @property
    def write_str(self):
        return self._proto.write_str

    @property
    def write_file(self):
        return self._proto.write_file

    @property
    def write_file_range(self):
        return self._proto.write_file_range

    @property
    def writer(self):
        return self._proto.writer


class SyncRSGIApp(RSGIApp):
    __slots__ = ['_inner']

    def __init__(self, app):
        self._inner = app

    def on_request(self, proto, scope):
        try:
            self._inner(scope, SyncRSGIProto(proto))
        except BaseException:
            logger.error('Application callable raised an exception', exc_info=True)
        finally:
            proto.close()


class SyncRSGIProto(_RSGIProtoWrapper):
    __slots__ = ['_proto']

    def watch(self):
        event = threading.Event()
        self._proto.watch(event.set)
        event.wait()

    def read(self):
        event = threading.Event()
        ret = []

        def _ok(*vals):
            ret.append((False, vals))
            event.set()

        def _err(err):
            ret.append((True, err))
            event.set()

        self._proto.read(_ok, _err)
        event.wait()

        is_err, res = ret[0]
        if is_err:
            raise res
        return res[0]

    def reader(self):
        return SyncRSGIReader(self._proto.reader())


class SyncRSGIReader:
    __slots__ = ['_reader']

    def __init__(self, reader):
        self._reader = reader

    def __iter__(self):
        event = threading.Event()
        ret = []

        def _ok(*vals):
            ret.append(vals)
            event.set()

        while True:
            self._reader.read(_ok)
            event.wait()

            data, eof = ret.pop()
            yield data

            if eof:
                break
            event.clear()


class AsyncioRSGIApp(RSGIApp):
    __slots__ = ['_inner', '_loop', '_tasks']

    def __init__(self, app, loop):
        self._inner = app
        self._loop = loop
        self._tasks = set()

    async def _http(self, proto, scope):
        try:
            await self._inner(scope, TonioRSGIProto(proto))
        except BaseException:
            logger.error('Application callable raised an exception', exc_info=True)
        finally:
            proto.close()

    def _crate_task(self, method, proto, scope):
        task = self._loop.create_task(method(proto, scope))
        self._tasks.add(task)
        task.add_done_callback(self._tasks.discard)

    def on_request(self, proto, scope):
        self._loop.call_soon_threadsafe(self._crate_task, self._http, proto, scope)


class AsyncIOResume:
    __slots__ = ['_fut']

    def __init__(self, fut):
        self._fut = fut

    def _ok(self, *res):
        self._fut.get_loop().call_soon_threadsafe(self._fut.set_result, res)

    def _err(self, err):
        self._fut.get_loop().call_soon_threadsafe(self._fut.set_exception, err)


class AsyncIORSGIProto(_RSGIProtoWrapper):
    __slots__ = ['_proto', '_app']

    def __init__(self, proto, app):
        self._proto = proto
        self._app = app

    async def watch(self):
        fut: asyncio.Future = self._app._loop.create_future()
        cb = AsyncIOResume(fut)
        cancel = self._proto.watch(cb._ok)
        try:
            await fut
        except asyncio.CancelledError:
            cancel()
            raise

    async def read(self):
        fut: asyncio.Future = self._app._loop.create_future()
        cb = AsyncIOResume(fut)
        cancel = self._proto.read(cb._ok, cb._err)
        try:
            res = await fut
        except asyncio.CancelledError:
            cancel()
            raise
        return res[0]

    def reader(self):
        return AsyncIORSGIReader(self._proto.reader(), self._app._loop)


class AsyncIORSGIReader:
    __slots__ = ['_reader', '_loop']

    def __init__(self, reader, loop):
        self._reader = reader
        self._loop = loop

    async def __aiter__(self):
        while True:
            fut: asyncio.Future = self._loop.create_future()
            cb = AsyncIOResume(fut)
            cancel = self._reader.read(cb._ok)
            try:
                res = await fut
            except asyncio.CancelledError:
                cancel()
                raise

            data, eof = res
            yield data

            if eof:
                break


class TonioRSGIApp(RSGIApp):
    __slots__ = ['_inner']

    def __init__(self, app):
        self._inner = app

    async def _http(self, proto, scope):
        try:
            await self._inner(scope, TonioRSGIProto(proto))
        except BaseException:
            logger.error('Application callable raised an exception', exc_info=True)
        finally:
            proto.close()

    def on_request(self, proto, scope):
        tonio.spawn.without_tracking(self._http(proto, scope))


class TonioRSGIProto(_RSGIProtoWrapper):
    __slots__ = ['_proto']

    async def watch(self):
        event = tonio.Event()
        cancel = self._proto.watch(event.set)
        try:
            await event.wait()
        except _TonioCancelled:
            cancel()
            raise

    async def read(self):
        event = tonio.Event()
        ret = TonioResult()

        def _ok(*vals):
            ret.store((False, vals))
            event.set()

        def _err(err):
            ret.store((True, err))
            event.set()

        cancel = self._proto.read(_ok, _err)
        try:
            await event.wait()
        except _TonioCancelled:
            cancel()
            raise

        is_err, res = ret.fetch()
        if is_err:
            raise res
        return res[0]

    def reader(self):
        return TonioRSGIReader(self._proto.reader())


class TonioRSGIReader:
    __slots__ = ['_reader']

    def __init__(self, reader):
        self._reader = reader

    async def __aiter__(self):
        event = tonio.Event()
        ret = TonioResult()

        def _ok(*vals):
            ret.store(vals)
            event.set()

        while True:
            cancel = self._reader.read(_ok)
            try:
                await event.wait()
            except _TonioCancelled:
                cancel()
                raise

            data, eof = ret.fetch()
            yield data

            if eof:
                break
            event.clear()
