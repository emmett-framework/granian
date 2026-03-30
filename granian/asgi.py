import asyncio
import time

from ._interop import App as _App
from .log import log_request_builder, logger


class AsyncIOLifespanProtocol:
    error_transition = 'Invalid lifespan state transition'

    def __init__(self, callable):
        self.callable = callable
        self.event_queue = asyncio.Queue()
        self.event_startup = asyncio.Event()
        self.event_shutdown = asyncio.Event()
        self.unsupported = False
        self.errored = False
        self.failure_startup = False
        self.failure_shutdown = False
        self.interrupt = False
        self.exc = None
        self.state = {}

    async def handle(self):
        try:
            await self.callable(
                {'type': 'lifespan', 'asgi': {'version': '3.0', 'spec_version': '2.3'}, 'state': self.state},
                self.receive,
                self.send,
            )
        except Exception as exc:
            self.errored = True
            self.exc = exc
            if self.failure_startup or self.failure_shutdown:
                return
            self.unsupported = True
            logger.warn(
                'ASGI Lifespan errored, continuing without Lifespan support '
                '(to avoid Lifespan completely use "asginl" interface)'
            )
        finally:
            self.event_startup.set()
            self.event_shutdown.set()

    async def startup(self):
        loop = asyncio.get_event_loop()
        _handler_task = loop.create_task(self.handle())

        await self.event_queue.put({'type': 'lifespan.startup'})
        await self.event_startup.wait()

        if self.failure_startup or (self.errored and not self.unsupported):
            self.interrupt = True

    async def shutdown(self):
        self.state.clear()

        if self.errored:
            return

        await self.event_queue.put({'type': 'lifespan.shutdown'})
        await self.event_shutdown.wait()

        if self.failure_shutdown or (self.errored and not self.unsupported):
            self.interrupt = True

    async def receive(self):
        return await self.event_queue.get()

    def _handle_startup_complete(self, message):
        assert not self.event_startup.is_set(), self.error_transition
        assert not self.event_shutdown.is_set(), self.error_transition
        self.event_startup.set()

    def _handle_startup_failed(self, message):
        assert not self.event_startup.is_set(), self.error_transition
        assert not self.event_shutdown.is_set(), self.error_transition
        self.event_startup.set()
        self.failure_startup = True
        if message.get('message'):
            logger.error(message['message'])

    def _handle_shutdown_complete(self, message):
        assert self.event_startup.is_set(), self.error_transition
        assert not self.event_shutdown.is_set(), self.error_transition
        self.event_shutdown.set()

    def _handle_shutdown_failed(self, message):
        assert self.event_startup.is_set(), self.error_transition
        assert not self.event_shutdown.is_set(), self.error_transition
        self.event_shutdown.set()
        self.failure_shutdown = True
        if message.get('message'):
            logger.error(message['message'])

    _event_handlers = {
        'lifespan.startup.complete': _handle_startup_complete,
        'lifespan.startup.failed': _handle_startup_failed,
        'lifespan.shutdown.complete': _handle_shutdown_complete,
        'lifespan.shutdown.failed': _handle_shutdown_failed,
    }

    async def send(self, message):
        handler = self._event_handlers[message['type']]
        handler(self, message)


def _build_access_logger(fmt):
    logger = log_request_builder(fmt)

    def _log_dict(scope):
        return {
            'addr_remote': scope['client'][0],
            'protocol': 'HTTP/' + scope['http_version'],
            'path': scope['path'],
            'qs': scope['query_string'].decode('latin-1'),
            'method': scope.get('method', '-'),
            'scheme': scope['scheme'],
        }

    def _access_log(rt, mt, scope, resp_code):
        logger(rt, mt, _log_dict(scope), resp_code)

    def _access_log_with_headers(rt, mt, scope, resp_code):
        data = _log_dict(scope)
        headers = {}
        for hname_b, hval_b in scope['headers']:
            hname = hname_b.decode('latin-1').lower()
            hval = hval_b.decode('latin-1')
            headers[hname] = hval
        data['headers'] = headers.get
        logger(rt, mt, data, resp_code)

    return _access_log_with_headers if logger.parse_headers else _access_log


class AsyncIOASGIApp(_App):
    __slots__ = ['_inner', '_loop', '_tasks', '_state', '_log', '_root_path']

    def __init__(self, app, loop, state, scope_opts=None, access_log_fmt=None):
        self._inner = app
        self._loop = loop
        self._tasks = set()
        self._log = _build_access_logger(access_log_fmt)
        self._state = state
        self._root_path = (scope_opts or {}).get('url_path_prefix') or ''

    async def _asgi(self, proto, scope):
        scope.update(root_path=self._root_path, state=self._state.copy())
        try:
            await self._inner(scope, proto.receive, proto.send)
        except BaseException:
            logger.error('Application callable raised an exception', exc_info=True)
        finally:
            proto._close()

    async def _http_wlog(self, proto, scope):
        scope.update(root_path=self._root_path, state=self._state.copy())
        rt, mt = time.time(), time.perf_counter()
        try:
            await self._inner(scope, proto.receive, proto.send)
        except BaseException:
            logger.error('Application callable raised an exception', exc_info=True)
        finally:
            proto._close()
            self._log(rt, mt, scope, proto.sent_response_code)

    def _ws_wlog(self, proto, scope):
        self._log(time.time(), time.perf_counter(), scope, 101)
        return self._asgi(proto, scope)

    def _crate_task(self, method, proto, scope):
        task = self._loop.create_task(method(proto, scope))
        self._tasks.add(task)
        task.add_done_callback(self._tasks.discard)

    def on_request(self, proto, scope):
        self._loop.call_soon_threadsafe(self._crate_task, self._asgi, proto, scope)

    def on_websocket(self, proto, scope):
        self._loop.call_soon_threadsafe(self._crate_task, self._asgi, proto, scope)

    def on_request_wlog(self, proto, scope):
        self._loop.call_soon_threadsafe(self._crate_task, self._http_wlog, proto, scope)

    def on_websocket_wlog(self, proto, scope):
        self._loop.call_soon_threadsafe(self._crate_task, self._ws_wlog, proto, scope)
