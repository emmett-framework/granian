import asyncio
import time
from functools import wraps

from .log import log_request_builder, logger


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


def _callback_wrapper(callback, scope_opts, state, access_log_fmt=None):
    root_url_path = scope_opts.get('url_path_prefix') or ''

    def _runner(scope, proto):
        scope.update(root_path=root_url_path, state=state.copy())
        return callback(scope, proto.receive, proto.send)

    async def _http_logger(scope, proto):
        rt, mt = time.time(), time.perf_counter()
        try:
            rv = await _runner(scope, proto)
        finally:
            access_log(rt, mt, scope, proto.sent_response_code)
        return rv

    async def _http_logger_with_resp_headers(scope, proto):
        rt, mt = time.time(), time.perf_counter()
        resp_headers_raw = []
        original_send = proto.send

        async def capturing_send(message):
            if message.get('type') == 'http.response.start':
                resp_headers_raw.extend(message.get('headers', ()))
            return await original_send(message)

        try:
            scope.update(root_path=root_url_path, state=state.copy())
            rv = await callback(scope, proto.receive, capturing_send)
        finally:
            access_log(rt, mt, scope, proto.sent_response_code, resp_headers_raw)
        return rv

    def _ws_logger(scope, proto):
        access_log(time.time(), time.perf_counter(), scope, 101)
        return _runner(scope, proto)

    def _logger(scope, proto):
        if scope['type'] == 'http':
            return _http_log(scope, proto)
        return _ws_logger(scope, proto)

    access_log, _needs_resp_headers = _build_access_logger(access_log_fmt)
    _http_log = _http_logger_with_resp_headers if _needs_resp_headers else _http_logger
    wrapper = _logger if access_log_fmt else _runner
    wraps(callback)(wrapper)

    return wrapper


def _build_access_logger(fmt):
    logger = log_request_builder(fmt)
    _needs_resp_headers = logger.needs_resp_headers

    def access_log(rt, mt, scope, resp_code, resp_headers_raw=()):
        user_agent = '-'
        headers_dict = {}
        for hname_b, hval_b in scope.get('headers', ()):
            hname = hname_b.decode('latin-1').lower()
            hval = hval_b.decode('latin-1')
            headers_dict[hname] = hval
            if hname == 'user-agent':
                user_agent = hval
        req = {
            'addr_remote': scope['client'][0],
            'protocol': 'HTTP/' + scope['http_version'],
            'path': scope['path'],
            'qs': scope['query_string'],
            'method': scope.get('method', '-'),
            'scheme': scope['scheme'],
            'user_agent': user_agent,
            'get_header': headers_dict.get,
        }
        if _needs_resp_headers:
            req['get_response_header'] = {
                hname_b.decode('latin-1').lower(): hval_b.decode('latin-1')
                for hname_b, hval_b in resp_headers_raw
            }.get
        logger(rt, mt, req, resp_code)

    return access_log, _needs_resp_headers
