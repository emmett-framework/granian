import sys
import threading
from functools import wraps
from typing import Any, Callable, Dict, Optional, Tuple

from .._futures import _future_watcher_wrapper, _new_cbscheduler
from .._granian import ASGIWorker, RSGIWorker, WorkerSignal, WorkerSignalSync, WSGIWorker
from .._loops import loops
from .._types import SSLCtx
from ..asgi import LifespanProtocol, _callback_wrapper as _asgi_call_wrap
from ..errors import ConfigurationError, FatalError
from ..rsgi import _callback_wrapper as _rsgi_call_wrap, _callbacks_from_target as _rsgi_cbs_from_target
from ..wsgi import _callback_wrapper as _wsgi_call_wrap
from .common import (
    WORKERS_METHODS,
    AbstractServer,
    AbstractWorker,
    HTTP1Settings,
    HTTP2Settings,
    HTTPModes,
    Interfaces,
    RuntimeModes,
    TaskImpl,
    logger,
)


class WorkerThread(AbstractWorker):
    _idl = 'TID'

    def __init__(self, parent, idx, target, args, sig):
        self._sig = sig
        super().__init__(parent, idx, target, args)

    @staticmethod
    def wrap_target(target):
        @wraps(target)
        def wrapped(worker_id, sig, callback, sock, loop_impl, *args, **kwargs):
            loop = loops.get(loop_impl)
            return target(worker_id, sig, callback, sock, loop, *args, **kwargs)

        return wrapped

    def _spawn(self, target, args):
        self.inner = threading.Thread(name='granian-worker', target=target, args=args)
        self._alive = True

    def _id(self):
        return self.inner.native_id

    def _watcher(self):
        self.inner.join()
        self._alive = False
        if not self.interrupt_by_parent:
            logger.error(f'Unexpected exit from worker-{self.idx + 1}')
            self.parent.interrupt_children.append(self.idx)
            self.parent.main_loop_interrupt.set()

    def terminate(self):
        self._alive = False
        self.interrupt_by_parent = True
        self._sig.set()

    def is_alive(self):
        if not self._alive:
            return False
        return self.inner.is_alive()


class MTServer(AbstractServer[WorkerThread]):
    @staticmethod
    @WorkerThread.wrap_target
    def _spawn_asgi_worker(
        worker_id: int,
        shutdown_event: Any,
        callback: Any,
        sock: Any,
        loop: Any,
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
        static_path: Optional[Tuple[str, str, Optional[str]]],
        log_access_fmt: Optional[str],
        ssl_ctx: SSLCtx,
        scope_opts: Dict[str, Any],
    ):
        wcallback = _future_watcher_wrapper(_asgi_call_wrap(callback, scope_opts, {}, log_access_fmt))

        worker = ASGIWorker(
            worker_id,
            sock,
            runtime_threads,
            runtime_blocking_threads,
            blocking_threads,
            blocking_threads_idle_timeout,
            backpressure,
            http_mode,
            http1_settings,
            http2_settings,
            websockets,
            static_path,
            *ssl_ctx,
        )
        serve = getattr(worker, WORKERS_METHODS[runtime_mode][sock.is_uds()])
        scheduler = _new_cbscheduler(loop, wcallback, impl_asyncio=task_impl == TaskImpl.asyncio)
        serve(scheduler, loop, shutdown_event)

    @staticmethod
    @WorkerThread.wrap_target
    def _spawn_asgi_lifespan_worker(
        worker_id: int,
        shutdown_event: Any,
        callback: Any,
        sock: Any,
        loop: Any,
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
        static_path: Optional[Tuple[str, str, Optional[str]]],
        log_access_fmt: Optional[str],
        ssl_ctx: SSLCtx,
        scope_opts: Dict[str, Any],
    ):
        lifespan_handler = LifespanProtocol(callback)
        wcallback = _future_watcher_wrapper(
            _asgi_call_wrap(callback, scope_opts, lifespan_handler.state, log_access_fmt)
        )

        loop.run_until_complete(lifespan_handler.startup())
        if lifespan_handler.interrupt:
            logger.error('ASGI lifespan startup failed', exc_info=lifespan_handler.exc)
            sys.exit(1)

        worker = ASGIWorker(
            worker_id,
            sock,
            runtime_threads,
            runtime_blocking_threads,
            blocking_threads,
            blocking_threads_idle_timeout,
            backpressure,
            http_mode,
            http1_settings,
            http2_settings,
            websockets,
            static_path,
            *ssl_ctx,
        )
        serve = getattr(worker, WORKERS_METHODS[runtime_mode][sock.is_uds()])
        scheduler = _new_cbscheduler(loop, wcallback, impl_asyncio=task_impl == TaskImpl.asyncio)
        serve(scheduler, loop, shutdown_event)
        loop.run_until_complete(lifespan_handler.shutdown())

    @staticmethod
    @WorkerThread.wrap_target
    def _spawn_rsgi_worker(
        worker_id: int,
        shutdown_event: Any,
        callback: Any,
        sock: Any,
        loop: Any,
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
        static_path: Optional[Tuple[str, str, Optional[str]]],
        log_access_fmt: Optional[str],
        ssl_ctx: SSLCtx,
        scope_opts: Dict[str, Any],
    ):
        callback, callback_init, callback_del = _rsgi_cbs_from_target(callback)
        wcallback = _future_watcher_wrapper(_rsgi_call_wrap(callback, log_access_fmt))
        callback_init(loop)

        worker = RSGIWorker(
            worker_id,
            sock,
            runtime_threads,
            runtime_blocking_threads,
            blocking_threads,
            blocking_threads_idle_timeout,
            backpressure,
            http_mode,
            http1_settings,
            http2_settings,
            websockets,
            static_path,
            *ssl_ctx,
        )
        serve = getattr(worker, WORKERS_METHODS[runtime_mode][sock.is_uds()])
        scheduler = _new_cbscheduler(loop, wcallback, impl_asyncio=task_impl == TaskImpl.asyncio)
        serve(scheduler, loop, shutdown_event)
        callback_del(loop)

    @staticmethod
    @WorkerThread.wrap_target
    def _spawn_wsgi_worker(
        worker_id: int,
        shutdown_event: Any,
        callback: Any,
        sock: Any,
        loop: Any,
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
        static_path: Optional[Tuple[str, str, Optional[str]]],
        log_access_fmt: Optional[str],
        ssl_ctx: SSLCtx,
        scope_opts: Dict[str, Any],
    ):
        wcallback = _wsgi_call_wrap(callback, scope_opts, log_access_fmt)

        worker = WSGIWorker(
            worker_id,
            sock,
            runtime_threads,
            runtime_blocking_threads,
            blocking_threads,
            blocking_threads_idle_timeout,
            backpressure,
            http_mode,
            http1_settings,
            http2_settings,
            static_path,
            *ssl_ctx,
        )
        serve = getattr(worker, WORKERS_METHODS[runtime_mode][sock.is_uds()])
        scheduler = _new_cbscheduler(loop, wcallback, impl_asyncio=task_impl == TaskImpl.asyncio)
        serve(scheduler, loop, shutdown_event)

    def _spawn_worker(self, idx, target, callback_loader) -> WorkerThread:
        sig = WorkerSignalSync(threading.Event()) if self.interface == Interfaces.WSGI else WorkerSignal()

        return WorkerThread(
            parent=self,
            idx=idx,
            target=target,
            args=(
                idx + 1,
                sig,
                callback_loader,
                self._shd,
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
                self.static_path,
                self.log_access_format if self.log_access else None,
                self.ssl_ctx,
                {'url_path_prefix': self.url_path_prefix},
            ),
            sig=sig,
        )

    def _check_gil(self):
        try:
            assert sys._is_gil_enabled() is False
        except Exception:
            logger.error('Cannot run a free-threaded Granian build with GIL enabled')
            raise FatalError('gil')

    def _serve(self, spawn_target, target_loader):
        target = target_loader()
        self._check_gil()
        self.startup(spawn_target, target)
        self._serve_loop(spawn_target, target)
        self.shutdown()

    def _serve_with_reloader(self, spawn_target, target_loader):
        raise NotImplementedError

    def serve(
        self,
        spawn_target: Optional[Callable[..., None]] = None,
        target_loader: Optional[Callable[..., Callable[..., Any]]] = None,
        wrap_loader: bool = True,
    ):
        logger.warning('free-threaded Python support is experimental!')

        if self.reload_on_changes:
            logger.error('The changes reloader is not supported on the free-threaded build')
            raise ConfigurationError('reload')

        if self.workers_rss:
            logger.error('The resource monitor is not supported on the free-threaded build')
            raise ConfigurationError('workers_max_rss')

        super().serve(spawn_target, target_loader, wrap_loader)
