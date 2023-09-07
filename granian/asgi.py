import asyncio
from functools import wraps

from ._granian import ASGIScope as Scope


class LifespanProtocol:
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

    async def handle(self):
        try:
            await self.callable(
                {'type': 'lifespan', 'asgi': {'version': '3.0', 'spec_version': '2.3'}}, self.receive, self.send
            )
        except Exception:
            self.errored = True
            if self.failure_startup or self.failure_shutdown:
                return
            self.unsupported = True
            # self.logger.error(msg, exc_info=exc)
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
        # if message.get("message"):
        #     self.logger.error(message["message"])

    def _handle_shutdown_complete(self, message):
        assert self.event_startup.is_set(), self.error_transition
        assert not self.event_shutdown.is_set(), self.error_transition
        self.event_shutdown.set()

    def _handle_shutdown_failed(self, message):
        assert self.event_startup.is_set(), self.error_transition
        assert not self.event_shutdown.is_set(), self.error_transition
        self.event_shutdown.set()
        self.failure_shutdown = True
        # if message.get("message"):
        #     self.logger.error(message["message"])

    _event_handlers = {
        'lifespan.startup.complete': _handle_startup_complete,
        'lifespan.startup.failed': _handle_startup_failed,
        'lifespan.shutdown.complete': _handle_shutdown_complete,
        'lifespan.shutdown.failed': _handle_shutdown_failed,
    }

    async def send(self, message):
        handler = self._event_handlers[message['type']]
        handler(self, message)


async def _noop_coro():
    return


def _send_wrapper(proto):
    @wraps(proto)
    def send(data):
        return proto(_noop_coro, data)

    return send


def _callback_wrapper(callback, scope_opts):
    root_url_path = scope_opts.get('url_path_prefix') or ''

    @wraps(callback)
    def wrapper(scope: Scope, proto):
        return callback(scope.as_dict(root_url_path), proto.receive, _send_wrapper(proto.send))

    return wrapper
