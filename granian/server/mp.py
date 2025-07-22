import multiprocessing
import socket
import sys
from functools import wraps
from typing import Any, Callable, Dict, Optional, Tuple

from .._futures import _future_watcher_wrapper, _new_cbscheduler
from .._granian import ASGIWorker, ProcInfoCollector, RSGIWorker, SocketHolder, WSGIWorker
from .._internal import load_env
from .._types import SSLCtx
from ..asgi import LifespanProtocol, _callback_wrapper as _asgi_call_wrap
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
    configure_logging,
    logger,
    setproctitle,
)


multiprocessing.allow_connection_pickling()


class WorkerProcess(AbstractWorker):
    _idl = 'PID'

    @staticmethod
    def wrap_target(target):
        @wraps(target)
        def wrapped(
            worker_id,
            process_name,
            callback_loader,
            sock,
            loop_impl,
            log_enabled,
            log_level,
            log_config,
            env_files,
            *args,
            **kwargs,
        ):
            from granian._loops import loops

            if process_name:
                setproctitle.setproctitle(f'{process_name} worker-{worker_id}')

            configure_logging(log_level, log_config, log_enabled)
            load_env(env_files)

            sock, _sso = sock
            if sys.platform == 'win32':
                sock = SocketHolder(_sso.fileno())

            loop = loops.get(loop_impl)
            callback = callback_loader()
            return target(worker_id, callback, sock, loop, *args, **kwargs)

        return wrapped

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
    @WorkerProcess.wrap_target
    def _spawn_asgi_worker(
        worker_id: int,
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
        from granian._signals import set_loop_signals

        wcallback = _future_watcher_wrapper(_asgi_call_wrap(callback, scope_opts, {}, log_access_fmt))
        shutdown_event = set_loop_signals(loop)

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
    @WorkerProcess.wrap_target
    def _spawn_asgi_lifespan_worker(
        worker_id: int,
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
        from granian._signals import set_loop_signals

        lifespan_handler = LifespanProtocol(callback)
        wcallback = _future_watcher_wrapper(
            _asgi_call_wrap(callback, scope_opts, lifespan_handler.state, log_access_fmt)
        )
        shutdown_event = set_loop_signals(loop)

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
    @WorkerProcess.wrap_target
    def _spawn_rsgi_worker(
        worker_id: int,
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
        from granian._signals import set_loop_signals

        callback, callback_init, callback_del = _rsgi_cbs_from_target(callback)
        wcallback = _future_watcher_wrapper(_rsgi_call_wrap(callback, log_access_fmt))
        shutdown_event = set_loop_signals(loop)
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
    @WorkerProcess.wrap_target
    def _spawn_wsgi_worker(
        worker_id: int,
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
        from granian._signals import set_sync_signals

        wcallback = _wsgi_call_wrap(callback, scope_opts, log_access_fmt)
        shutdown_event = set_sync_signals()

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

    def _init_shared_socket(self):
        super()._init_shared_socket()
        sock = socket.socket(fileno=self._sfd)
        sock.set_inheritable(True)
        self._sso = sock

    def _write_pidfile(self):
        super()._write_pidfile()
        self._rss_collector = ProcInfoCollector()

    def _unlink_pidfile(self):
        self._sso.detach()
        super()._unlink_pidfile()

    def _handle_rss_signal(self, spawn_target, target_loader):
        wpids = {wrk._id(): wrk for wrk in self.wrks}
        try:
            rss_data = self._rss_collector.memory(list(wpids.keys()))
        except Exception:
            logger.warning('Unable to collect resource usage for workers')
            return
        logger.debug(f'Collected resource usages for workers: {rss_data}')
        to_restart = []
        for wpid, wmem in rss_data.items():
            if wmem >= self.workers_rss:
                wrk = wpids[wpid]
                logger.info(f'worker-{wrk.idx + 1} RSS over threshold, gracefully respawning..')
                to_restart.append(wrk.idx)
        if to_restart:
            self._respawn_workers(to_restart, spawn_target, target_loader, delay=self.respawn_interval)

    def _spawn_worker(self, idx, target, callback_loader) -> WorkerProcess:
        return WorkerProcess(
            parent=self,
            idx=idx,
            target=target,
            args=(
                idx + 1,
                self.process_name,
                callback_loader,
                (self._shd, self._sso),
                self.loop,
                self.log_enabled,
                self.log_level,
                self.log_config,
                self.env_files,
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
                    f'Configuration allows spawning up to {self.blocking_threads} Python threads, '
                    'which seems quite high compared to the number of CPU cores available. '
                    'Consider reviewing your configuration and using `backpressure` to limit '
                    'the concurrency on the Python interpreter. '
                    'If this configuration is intentional, you can safely ignore this message.'
                )

        super().serve(spawn_target, target_loader, wrap_loader)
