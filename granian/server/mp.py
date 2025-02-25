import multiprocessing
import socket
import sys
from typing import Any, Callable, Dict, Optional, Tuple

from .._futures import _future_watcher_wrapper, _new_cbscheduler
from .._granian import ASGIWorker, RSGIWorker, WSGIWorker
from ..asgi import LifespanProtocol, _callback_wrapper as _asgi_call_wrap
from ..rsgi import _callback_wrapper as _rsgi_call_wrap
from ..wsgi import _callback_wrapper as _wsgi_call_wrap
from .common import (
    AbstractServer,
    AbstractWorker,
    HTTP1Settings,
    HTTP2Settings,
    HTTPModes,
    Interfaces,
    LogLevels,
    Loops,
    RuntimeModes,
    TaskImpl,
    configure_logging,
    logger,
    setproctitle,
)


multiprocessing.allow_connection_pickling()


class WorkerProcess(AbstractWorker):
    _idl = 'PID'

    def _spawn(self, target, args):
        self.inner = multiprocessing.get_context().Process(name='granian-worker', target=target, args=args)

    def _id(self):
        return self.inner.pid

    def terminate(self):
        self.interrupt_by_parent = True
        self.inner.terminate()

    def kill(self):
        self.interrupt_by_parent = True
        self.inner.kill()


class MPServer(AbstractServer[WorkerProcess]):
    @staticmethod
    def _spawn_asgi_worker(
        worker_id: int,
        process_name: Optional[str],
        callback_loader: Callable[..., Any],
        sock: socket.socket,
        loop_impl: Loops,
        runtime_mode: RuntimeModes,
        runtime_threads: int,
        runtime_blocking_threads: Optional[int],
        blocking_threads: int,
        blocking_threads_idle_timeout: int,
        backpressure: int,
        task_impl: TaskImpl,
        http_mode: HTTPModes,
        http1_settings: Optional[HTTP1Settings],
        http2_settings: Optional[HTTP2Settings],
        websockets: bool,
        log_enabled: bool,
        log_level: LogLevels,
        log_config: Dict[str, Any],
        log_access_fmt: Optional[str],
        ssl_ctx: Tuple[bool, Optional[str], Optional[str], Optional[str]],
        scope_opts: Dict[str, Any],
    ):
        from granian._loops import loops
        from granian._signals import set_loop_signals

        if process_name:
            setproctitle.setproctitle(f'{process_name} worker-{worker_id}')
        configure_logging(log_level, log_config, log_enabled)

        loop = loops.get(loop_impl)
        callback = callback_loader()
        shutdown_event = set_loop_signals(loop)
        wcallback = _asgi_call_wrap(callback, scope_opts, {}, log_access_fmt)

        worker = ASGIWorker(
            worker_id,
            sock.fileno(),
            runtime_threads,
            runtime_blocking_threads,
            blocking_threads,
            blocking_threads_idle_timeout,
            backpressure,
            http_mode,
            http1_settings,
            http2_settings,
            websockets,
            *ssl_ctx,
        )
        serve = getattr(worker, {RuntimeModes.mt: 'serve_mtr', RuntimeModes.st: 'serve_str'}[runtime_mode])
        scheduler = _new_cbscheduler(
            loop, _future_watcher_wrapper(wcallback), impl_asyncio=task_impl == TaskImpl.asyncio
        )
        serve(scheduler, loop, shutdown_event)

    @staticmethod
    def _spawn_asgi_lifespan_worker(
        worker_id: int,
        process_name: Optional[str],
        callback_loader: Callable[..., Any],
        sock: socket.socket,
        loop_impl: Loops,
        runtime_mode: RuntimeModes,
        runtime_threads: int,
        runtime_blocking_threads: Optional[int],
        blocking_threads: int,
        blocking_threads_idle_timeout: int,
        backpressure: int,
        task_impl: TaskImpl,
        http_mode: HTTPModes,
        http1_settings: Optional[HTTP1Settings],
        http2_settings: Optional[HTTP2Settings],
        websockets: bool,
        log_enabled: bool,
        log_level: LogLevels,
        log_config: Dict[str, Any],
        log_access_fmt: Optional[str],
        ssl_ctx: Tuple[bool, Optional[str], Optional[str], Optional[str]],
        scope_opts: Dict[str, Any],
    ):
        from granian._loops import loops
        from granian._signals import set_loop_signals

        if process_name:
            setproctitle.setproctitle(f'{process_name} worker-{worker_id}')
        configure_logging(log_level, log_config, log_enabled)

        loop = loops.get(loop_impl)
        callback = callback_loader()
        lifespan_handler = LifespanProtocol(callback)

        loop.run_until_complete(lifespan_handler.startup())
        if lifespan_handler.interrupt:
            logger.error('ASGI lifespan startup failed', exc_info=lifespan_handler.exc)
            sys.exit(1)

        shutdown_event = set_loop_signals(loop)
        wcallback = _asgi_call_wrap(callback, scope_opts, lifespan_handler.state, log_access_fmt)

        worker = ASGIWorker(
            worker_id,
            sock.fileno(),
            runtime_threads,
            runtime_blocking_threads,
            blocking_threads,
            blocking_threads_idle_timeout,
            backpressure,
            http_mode,
            http1_settings,
            http2_settings,
            websockets,
            *ssl_ctx,
        )
        serve = getattr(worker, {RuntimeModes.mt: 'serve_mtr', RuntimeModes.st: 'serve_str'}[runtime_mode])
        scheduler = _new_cbscheduler(
            loop, _future_watcher_wrapper(wcallback), impl_asyncio=task_impl == TaskImpl.asyncio
        )
        serve(scheduler, loop, shutdown_event)
        loop.run_until_complete(lifespan_handler.shutdown())

    @staticmethod
    def _spawn_rsgi_worker(
        worker_id: int,
        process_name: Optional[str],
        callback_loader: Callable[..., Any],
        sock: socket.socket,
        loop_impl: Loops,
        runtime_mode: RuntimeModes,
        runtime_threads: int,
        runtime_blocking_threads: Optional[int],
        blocking_threads: int,
        blocking_threads_idle_timeout: int,
        backpressure: int,
        task_impl: TaskImpl,
        http_mode: HTTPModes,
        http1_settings: Optional[HTTP1Settings],
        http2_settings: Optional[HTTP2Settings],
        websockets: bool,
        log_enabled: bool,
        log_level: LogLevels,
        log_config: Dict[str, Any],
        log_access_fmt: Optional[str],
        ssl_ctx: Tuple[bool, Optional[str], Optional[str], Optional[str]],
        scope_opts: Dict[str, Any],
    ):
        from granian._loops import loops
        from granian._signals import set_loop_signals

        if process_name:
            setproctitle.setproctitle(f'{process_name} worker-{worker_id}')
        configure_logging(log_level, log_config, log_enabled)

        loop = loops.get(loop_impl)
        target = callback_loader()
        callback = getattr(target, '__rsgi__') if hasattr(target, '__rsgi__') else target
        callback_init = (
            getattr(target, '__rsgi_init__') if hasattr(target, '__rsgi_init__') else lambda *args, **kwargs: None
        )
        callback_del = (
            getattr(target, '__rsgi_del__') if hasattr(target, '__rsgi_del__') else lambda *args, **kwargs: None
        )
        callback = _rsgi_call_wrap(callback, log_access_fmt)
        shutdown_event = set_loop_signals(loop)
        callback_init(loop)

        worker = RSGIWorker(
            worker_id,
            sock.fileno(),
            runtime_threads,
            runtime_blocking_threads,
            blocking_threads,
            blocking_threads_idle_timeout,
            backpressure,
            http_mode,
            http1_settings,
            http2_settings,
            websockets,
            *ssl_ctx,
        )
        serve = getattr(worker, {RuntimeModes.mt: 'serve_mtr', RuntimeModes.st: 'serve_str'}[runtime_mode])
        scheduler = _new_cbscheduler(
            loop, _future_watcher_wrapper(callback), impl_asyncio=task_impl == TaskImpl.asyncio
        )
        serve(scheduler, loop, shutdown_event)
        callback_del(loop)

    @staticmethod
    def _spawn_wsgi_worker(
        worker_id: int,
        process_name: Optional[str],
        callback_loader: Callable[..., Any],
        sock: socket.socket,
        loop_impl: Loops,
        runtime_mode: RuntimeModes,
        runtime_threads: int,
        runtime_blocking_threads: Optional[int],
        blocking_threads: int,
        blocking_threads_idle_timeout: int,
        backpressure: int,
        task_impl: TaskImpl,
        http_mode: HTTPModes,
        http1_settings: Optional[HTTP1Settings],
        http2_settings: Optional[HTTP2Settings],
        websockets: bool,
        log_enabled: bool,
        log_level: LogLevels,
        log_config: Dict[str, Any],
        log_access_fmt: Optional[str],
        ssl_ctx: Tuple[bool, Optional[str], Optional[str], Optional[str]],
        scope_opts: Dict[str, Any],
    ):
        from granian._loops import loops
        from granian._signals import set_sync_signals

        if process_name:
            setproctitle.setproctitle(f'{process_name} worker-{worker_id}')
        configure_logging(log_level, log_config, log_enabled)

        loop = loops.get(loop_impl)
        callback = callback_loader()
        shutdown_event = set_sync_signals()

        worker = WSGIWorker(
            worker_id,
            sock.fileno(),
            runtime_threads,
            runtime_blocking_threads,
            blocking_threads,
            blocking_threads_idle_timeout,
            backpressure,
            http_mode,
            http1_settings,
            http2_settings,
            *ssl_ctx,
        )
        serve = getattr(worker, {RuntimeModes.mt: 'serve_mtr', RuntimeModes.st: 'serve_str'}[runtime_mode])
        scheduler = _new_cbscheduler(
            loop, _wsgi_call_wrap(callback, scope_opts, log_access_fmt), impl_asyncio=task_impl == TaskImpl.asyncio
        )
        serve(scheduler, loop, shutdown_event)

    def _init_shared_socket(self):
        super()._init_shared_socket()
        sock = socket.socket(fileno=self._sfd)
        sock.set_inheritable(True)
        self._sso = sock

    def _spawn_worker(self, idx, target, callback_loader) -> WorkerProcess:
        return WorkerProcess(
            parent=self,
            idx=idx,
            target=target,
            args=(
                idx + 1,
                self.process_name,
                callback_loader,
                self._sso,
                self.loop,
                self.runtime_mode,
                self.runtime_threads,
                self.runtime_blocking_threads,
                self.blocking_threads,
                self.blocking_threads_idle_timeout,
                self.backpressure,
                self.task_impl,
                self.http,
                self.http1_settings,
                self.http2_settings,
                self.websockets,
                self.log_enabled,
                self.log_level,
                self.log_config,
                self.log_access_format if self.log_access else None,
                self.ssl_ctx,
                {'url_path_prefix': self.url_path_prefix},
            ),
        )

    def serve(
        self,
        spawn_target: Optional[Callable[..., None]] = None,
        target_loader: Optional[Callable[..., Callable[..., Any]]] = None,
        wrap_loader: bool = True,
    ):
        if self.interface == Interfaces.WSGI:
            if self.blocking_threads > (multiprocessing.cpu_count() * 2 + 1):
                logger.warning(
                    f'Configuration allow to spawn up to {self.blocking_threads} Python threads, '
                    'which appears to be quite high compared to the amount of CPU cores available. '
                    'Considering reviewing your configuration and use `backpressure` to limit the amount '
                    'of concurrency on the Python interpreter. '
                    'If this is intended, you can safely ignore this message.'
                )

        super().serve(spawn_target, target_loader, wrap_loader)
