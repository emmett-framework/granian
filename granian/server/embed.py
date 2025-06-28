import asyncio
import multiprocessing
import time
from functools import wraps
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

from .._futures import _future_watcher_wrapper, _new_cbscheduler
from .._granian import ASGIWorker, RSGIWorker, WorkerSignal
from .._imports import dotenv
from .._types import SSLCtx
from ..asgi import LifespanProtocol, _callback_wrapper as _asgi_call_wrap
from ..errors import ConfigurationError, FatalError
from ..rsgi import _callback_wrapper as _rsgi_call_wrap, _callbacks_from_target as _rsgi_cbs_from_target
from .common import (
    _PY_312,
    _PYV,
    AbstractServer,
    AbstractWorker,
    HTTP1Settings,
    HTTP2Settings,
    HTTPModes,
    Interfaces,
    LogLevels,
    TaskImpl,
    load_env,
    logger,
)


class AsyncWorker(AbstractWorker):
    def __init__(self, parent, idx, target, args, sig):
        self._sig = sig
        self._loop = asyncio.get_event_loop()
        self._task = None
        self._wtask = None
        super().__init__(parent, idx, target, args)

    @staticmethod
    def wrap_target(target):
        @wraps(target)
        def wrapped(worker_id, sig, callback, sock, *args, **kwargs):
            loop = asyncio.get_event_loop()
            return target(worker_id, sig, callback, sock, loop, *args, **kwargs)

        return wrapped

    def _spawn(self, target, args):
        self._task = self._loop.create_task(target(*args))
        self._alive = True

    def _id(self):
        return id(self._task)

    async def _watcher(self):
        try:
            await self._task
        except BaseException:
            pass
        if not self.interrupt_by_parent:
            logger.error(f'Unexpected exit from worker-{self.idx + 1}')
            self.parent.interrupt_children.append(self.idx)
            self.parent.main_loop_interrupt.set()

    def _watch(self):
        self._wtask = self._loop.create_task(self._watcher())

    def start(self):
        logger.info(f'Spawning worker-{self.idx + 1} with {self._idl}: {self._id()}')
        self._watch()

    def is_alive(self):
        if not self._alive:
            return False
        return not self._task.done()

    def terminate(self):
        self._alive = False
        self.interrupt_by_parent = True
        self._sig.set()

    def kill(self):
        self._alive = False
        self.interrupt_by_parent = True
        self._task.cancel()

    def join(self, timeout=None):
        return asyncio.wait_for(self._task, timeout=timeout)


class Server(AbstractServer[AsyncWorker]):
    def __init__(
        self,
        target: Any,
        address: str = '127.0.0.1',
        port: int = 8000,
        interface: Interfaces = Interfaces.RSGI,
        blocking_threads: Optional[int] = None,
        blocking_threads_idle_timeout: int = 30,
        runtime_threads: int = 1,
        runtime_blocking_threads: Optional[int] = None,
        task_impl: TaskImpl = TaskImpl.asyncio,
        http: HTTPModes = HTTPModes.auto,
        websockets: bool = True,
        backlog: int = 128,
        backpressure: Optional[int] = None,
        http1_settings: Optional[HTTP1Settings] = None,
        http2_settings: Optional[HTTP2Settings] = None,
        log_enabled: bool = True,
        log_level: LogLevels = LogLevels.info,
        log_dictconfig: Optional[Dict[str, Any]] = None,
        log_access: bool = False,
        log_access_format: Optional[str] = None,
        ssl_cert: Optional[Path] = None,
        ssl_key: Optional[Path] = None,
        ssl_key_password: Optional[str] = None,
        ssl_ca: Optional[Path] = None,
        ssl_crl: Optional[List[Path]] = None,
        ssl_client_verify: bool = False,
        url_path_prefix: Optional[str] = None,
        factory: bool = False,
        static_path_route: str = '/static',
        static_path_mount: Optional[Path] = None,
        static_path_expires: int = 86400,
    ):
        super().__init__(
            target=target,
            address=address,
            port=port,
            interface=interface,
            blocking_threads=blocking_threads,
            blocking_threads_idle_timeout=blocking_threads_idle_timeout,
            runtime_threads=runtime_threads,
            runtime_blocking_threads=runtime_blocking_threads,
            task_impl=task_impl,
            http=http,
            websockets=websockets,
            backlog=backlog,
            backpressure=backpressure,
            http1_settings=http1_settings,
            http2_settings=http2_settings,
            log_enabled=log_enabled,
            log_level=log_level,
            log_dictconfig=log_dictconfig,
            log_access=log_access,
            log_access_format=log_access_format,
            ssl_cert=ssl_cert,
            ssl_key=ssl_key,
            ssl_key_password=ssl_key_password,
            ssl_ca=ssl_ca,
            ssl_crl=ssl_crl,
            ssl_client_verify=ssl_client_verify,
            url_path_prefix=url_path_prefix,
            factory=factory,
            static_path_route=static_path_route,
            static_path_mount=static_path_mount,
            static_path_expires=static_path_expires,
        )
        self.main_loop_interrupt = asyncio.Event()

    def _spawn_worker(self, idx, target, callback_loader) -> AsyncWorker:
        sig = WorkerSignal()

        return AsyncWorker(
            parent=self,
            idx=idx,
            target=target,
            args=(
                idx + 1,
                sig,
                callback_loader,
                self._shd,
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

    @staticmethod
    @AsyncWorker.wrap_target
    async def _spawn_asgi_worker(
        worker_id: int,
        shutdown_event: Any,
        callback: Any,
        sock: Any,
        loop: Any,
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
        static_path: Optional[Tuple[str, str, str]],
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
        scheduler = _new_cbscheduler(loop, wcallback, impl_asyncio=task_impl == TaskImpl.asyncio)
        await worker.serve_async(scheduler, loop, shutdown_event)

    @staticmethod
    @AsyncWorker.wrap_target
    async def _spawn_asgi_lifespan_worker(
        worker_id: int,
        shutdown_event: Any,
        callback: Any,
        sock: Any,
        loop: Any,
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
        static_path: Optional[Tuple[str, str, str]],
        log_access_fmt: Optional[str],
        ssl_ctx: SSLCtx,
        scope_opts: Dict[str, Any],
    ):
        lifespan_handler = LifespanProtocol(callback)
        wcallback = _future_watcher_wrapper(
            _asgi_call_wrap(callback, scope_opts, lifespan_handler.state, log_access_fmt)
        )

        await lifespan_handler.startup()
        if lifespan_handler.interrupt:
            logger.error('ASGI lifespan startup failed', exc_info=lifespan_handler.exc)
            raise FatalError('ASGI lifespan startup')

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
        scheduler = _new_cbscheduler(loop, wcallback, impl_asyncio=task_impl == TaskImpl.asyncio)
        await worker.serve_async(scheduler, loop, shutdown_event)
        await lifespan_handler.shutdown()

    @staticmethod
    @AsyncWorker.wrap_target
    async def _spawn_rsgi_worker(
        worker_id: int,
        shutdown_event: Any,
        callback: Any,
        sock: Any,
        loop: Any,
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
        static_path: Optional[Tuple[str, str, str]],
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
        scheduler = _new_cbscheduler(loop, wcallback, impl_asyncio=task_impl == TaskImpl.asyncio)
        await worker.serve_async(scheduler, loop, shutdown_event)
        callback_del(loop)

    async def _respawn_workers(self, workers, spawn_target, target_loader, delay: float = 0):
        for idx in workers:
            self.respawned_wrks[idx] = time.time()
            logger.info(f'Respawning worker-{idx + 1}')
            old_wrk = self.wrks.pop(idx)
            wrk = self._spawn_worker(idx=idx, target=spawn_target, callback_loader=target_loader)
            wrk.start()
            self.wrks.insert(idx, wrk)
            await asyncio.sleep(delay)
            logger.info(f'Stopping old worker-{idx + 1}')
            old_wrk.terminate()
            await old_wrk.join(self.workers_kill_timeout)
            if self.workers_kill_timeout:
                # the worker might still be reported alive after `join`, let's context switch
                if old_wrk.is_alive():
                    await asyncio.sleep(0.001)
                if old_wrk.is_alive():
                    logger.warning(f'Killing old worker-{idx + 1} after it refused to gracefully stop')
                    old_wrk.kill()
                    await old_wrk.join()

    async def _stop_workers(self):
        for wrk in self.wrks:
            wrk.terminate()

        for wrk in self.wrks:
            await wrk.join(self.workers_kill_timeout)
            if self.workers_kill_timeout:
                if wrk.is_alive():
                    logger.warning(f'Killing worker-{wrk.idx} after it refused to gracefully stop')
                    wrk.kill()

        self.wrks.clear()

    def startup(self, spawn_target, target_loader):
        logger.info('Starting granian (embedded)')
        self._init_shared_socket()
        proto = 'https' if self.ssl_ctx[0] else 'http'
        logger.info(f'Listening at: {proto}://{self.bind_addr}:{self.bind_port}')

        load_env(self.env_files)
        self._call_hooks(self.hooks_startup)
        self._spawn_workers(spawn_target, target_loader)

    async def _serve_loop(self, spawn_target, target_loader):
        while True:
            await self.main_loop_interrupt.wait()
            if self.interrupt_signal:
                break

            if self.interrupt_children:
                break

            if self.reload_signal:
                await self._reload(spawn_target, target_loader)

    async def shutdown(self, exit_code=0):
        logger.info('Shutting down granian')
        self._call_hooks(self.hooks_shutdown)
        await self._stop_workers()

    async def _serve(self, spawn_target, target_loader):
        target = target_loader()
        self.startup(spawn_target, target)
        await self._serve_loop(spawn_target, target)
        await self.shutdown()

    async def serve(self, spawn_target: Optional[Callable[..., None]] = None):
        def target_loader(*args, **kwargs):
            if self.factory:
                return self.target()
            return self.target

        default_spawners = {
            Interfaces.ASGI: self._spawn_asgi_lifespan_worker,
            Interfaces.ASGINL: self._spawn_asgi_worker,
            Interfaces.RSGI: self._spawn_rsgi_worker,
        }

        logger.warning('Embedded server is experimental!')

        if self.interface == Interfaces.WSGI:
            logger.error('WSGI is not supported in embedded mode')
            raise ConfigurationError('interface')

        if self.reload_on_changes:
            logger.error('The changes reloader is not supported in embedded mode')
            raise ConfigurationError('reload')

        if not spawn_target:
            spawn_target = default_spawners[self.interface]

        if self.blocking_threads > 1:
            logger.error('Blocking threads > 1 is not supported on ASGI and RSGI')
            raise ConfigurationError('blocking_threads')

        if self.websockets:
            if self.http == HTTPModes.http2:
                logger.info('Websockets are not supported on HTTP/2 only, ignoring')

        if self.env_files and dotenv is None:
            logger.error('Environment file(s) usage requires the granian[dotenv] extra')
            raise ConfigurationError('env_files')

        if self.blocking_threads_idle_timeout < 10 or self.blocking_threads_idle_timeout > 600:
            logger.error('Blocking threads idle timeout must be between 10 and 600 seconds')
            raise ConfigurationError('blocking_threads_idle_timeout')

        cpus = multiprocessing.cpu_count()
        if self.runtime_threads > cpus:
            logger.warning(
                'Configured number of Rust threads appears to be too high given the amount of CPU cores available. '
                'Mind that Rust threads are not involved in Python code execution, and they almost never be the '
                'limiting factor in scaling. Consider configuring the amount of blocking threads instead'
            )

        if self.task_impl == TaskImpl.rust:
            if _PYV >= _PY_312:
                self.task_impl = TaskImpl.asyncio
                logger.warning('Rust task implementation is not available on Python >= 3.12, falling back to asyncio')
            else:
                logger.warning('Rust task implementation is experimental!')

        await self._serve(spawn_target, target_loader)

    def stop(self):
        self.signal_handler_interrupt()

    def reload(self):
        self.signal_handler_reload()
