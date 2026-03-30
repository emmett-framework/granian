import abc
import asyncio
import threading


try:
    import tonio.colored as tonio
except Exception:
    tonio = None

from ._granian import RSGIApp as _RSGIApp


class RSGIApp(abc.ABC):
    def rsgi(self):
        return _RSGIApp(self)

    @abc.abstractmethod
    def on_request(self, proto, scope): ...

    # @abc.abstractmethod
    # def on_websocket(self, proto, scope):
    #     ...


class SyncRSGIApp(RSGIApp):
    __slots__ = ['_inner']

    def __init__(self, app):
        self._inner = app

    def on_request(self, proto, scope):
        self._inner(scope, SyncRSGIProto(proto))


class SyncRSGIProto:
    __slots__ = ['_proto']

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

    def watch(self):
        event = threading.Event()
        self._proto.watch(event.set)
        event.wait()

    def read(self):
        event = threading.Event()
        ret = []

        def cb(*vals):
            ret.append(vals)
            event.set()

        self._proto.read(cb)
        event.wait()
        data, eof = ret[0]
        return data

    def reader(self):
        return SyncRSGIReader(self._proto.reader())


class SyncRSGIReader:
    __slots__ = ['_reader']

    def __init__(self, reader):
        self._reader = reader

    def __iter__(self):
        def cb(event, ret):
            def inner(*vals):
                ret.append(vals)
                event.set()

            return inner

        while True:
            event = threading.Event()
            ret = []
            self._reader.read(cb(event, ret))
            event.wait()
            data, eof = ret[0]
            yield data
            if eof:
                break


class AsyncioRSGIApp(RSGIApp):
    __slots__ = ['_inner', '_loop', '_tasks']

    def __init__(self, app, loop):
        self._inner = app
        self._loop = loop
        self._tasks = set()

    def _crate_task(self, proto, scope):
        task = self._loop.create_task(self._inner(scope, proto))
        self._tasks.add(task)
        task.add_done_callback(self._tasks.discard)

    def on_request(self, proto, scope):
        self._loop.call_soon_threadsafe(self._crate_task, AsyncIORSGIProto(proto, self), scope)


class AsyncIOResume:
    __slots__ = ['_fut']

    def __init__(self, fut):
        self._fut = fut

    def __call__(self, *res):
        self._fut.get_loop().call_soon_threadsafe(self._fut.set_result, res)

    # TODO: custom future with on_data, on_err?


class AsyncIORSGIProto:
    __slots__ = ['_proto', '_app']

    def __init__(self, proto, app):
        self._proto = proto
        self._app = app

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

    async def watch(self):
        fut: asyncio.Future = self._app._loop.create_future()
        cb = AsyncIOResume(fut)
        cancel = self._proto.watch(cb)
        try:
            await fut
        except asyncio.CancelledError:
            cancel()

    async def read(self):
        fut: asyncio.Future = self._app._loop.create_future()
        cb = AsyncIOResume(fut)
        cancel = self._proto.read(cb)
        try:
            await fut
        except asyncio.CancelledError:
            cancel()
        data, eof = fut.result()
        return data

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
            cancel = self._reader.read(cb)
            try:
                await fut
            except asyncio.CancelledError:
                cancel()
            data, eof = fut.result()
            yield data
            if eof:
                break


class TonioRSGIApp(RSGIApp):
    __slots__ = ['_inner']

    def __init__(self, app):
        self._inner = app

    def on_request(self, proto, scope):
        tonio.spawn.without_tracking(self._inner(scope, TonioRSGIProto(proto)))


class TonioRSGIProto:
    __slots__ = ['_proto']

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

    async def watch(self):
        event = tonio.Event()
        cancel = self._proto.watch(event.set)
        try:
            await event.wait()
        except tonio.CancelledError:
            cancel()

    async def read(self):
        event = tonio.Event()
        ret = []

        def cb(*vals):
            ret.append(vals)
            event.set()

        cancel = self._proto.read(cb)
        try:
            await event.wait()
        except tonio.CancelledError:
            cancel()
        data, _ = ret[0]
        return data

    def reader(self):
        return TonioRSGIReader(self._proto.reader())


class TonioRSGIReader:
    __slots__ = ['_reader']

    def __init__(self, reader):
        self._reader = reader

    async def __aiter__(self):
        def cb(event, ret):
            def inner(*vals):
                ret.append(vals)
                event.set()

            return inner

        while True:
            event = tonio.Event()
            ret = []
            cancel = self._reader.read(cb(event, ret))
            try:
                await event.wait()
            except tonio.CancelledError:
                cancel()
            data, eof = ret[0]
            yield data
            if eof:
                break
