from __future__ import annotations

import contextvars
import multiprocessing
import signal
import socket
import ssl
import sys
import threading
from functools import partial
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional

import watchfiles

from ._futures import future_watcher_wrapper
from ._granian import ASGIWorker, RSGIWorker, WSGIWorker
from ._internal import load_target
from .asgi import LifespanProtocol, _callback_wrapper as _asgi_call_wrap
from .constants import HTTPModes, Interfaces, Loops, ThreadModes
from .log import LogLevels, configure_logging, logger
from .net import SocketHolder
from .wsgi import _callback_wrapper as _wsgi_call_wrap


multiprocessing.allow_connection_pickling()


class Worker:
    def __init__(self, parent: Granian, idx: int, target: Any, args: Any):
        self.parent = parent
        self.idx = idx
        self.interrupt_by_parent = False
        self._spawn(target, args)

    def _spawn(self, target, args):
        self.proc = multiprocessing.get_context().Process(name='granian-worker', target=target, args=args)

    def _watcher(self):
        self.proc.join()
        if not self.interrupt_by_parent:
            logger.error(f'Unexpected exit from worker-{self.idx + 1}')
            self.parent.interrupt_child = self.idx
            self.parent.main_loop_interrupt.set()

    def _watch(self):
        watcher = threading.Thread(target=self._watcher)
        watcher.start()

    def start(self):
        self.proc.start()
        logger.info(f'Spawning worker-{self.idx + 1} with pid: {self.proc.pid}')
        self._watch()

    def terminate(self):
        self.interrupt_by_parent = True
        self.proc.terminate()

    def join(self, timeout=None):
        self.proc.join(timeout=timeout)


class Granian:
    SIGNALS = {signal.SIGINT, signal.SIGTERM}

    def __init__(
        self,
        target: str,
        address: str = '127.0.0.1',
        port: int = 8000,
        interface: Interfaces = Interfaces.RSGI,
        workers: int = 1,
        threads: int = 1,
        pthreads: int = 1,
        threading_mode: ThreadModes = ThreadModes.workers,
        loop: Loops = Loops.auto,
        loop_opt: bool = False,
        http: HTTPModes = HTTPModes.auto,
        websockets: bool = True,
        backlog: int = 1024,
        http1_buffer_size: int = 65535,
        log_enabled: bool = True,
        log_level: LogLevels = LogLevels.info,
        log_dictconfig: Optional[Dict[str, Any]] = None,
        ssl_cert: Optional[Path] = None,
        ssl_key: Optional[Path] = None,
        url_path_prefix: Optional[str] = None,
        reload: bool = False,
    ):
        self.target = target
        self.bind_addr = address
        self.bind_port = port
        self.interface = interface
        self.workers = max(1, workers)
        self.threads = max(1, threads)
        self.pthreads = max(1, pthreads)
        self.threading_mode = threading_mode
        self.loop = loop
        self.loop_opt = loop_opt
        self.http = http
        self.websockets = websockets
        self.backlog = max(128, backlog)
        self.http1_buffer_size = http1_buffer_size
        self.log_enabled = log_enabled
        self.log_level = log_level
        self.log_config = log_dictconfig
        self.url_path_prefix = url_path_prefix
        self.reload_on_changes = reload

        configure_logging(self.log_level, self.log_config, self.log_enabled)

        self.build_ssl_context(ssl_cert, ssl_key)
        self._shd = None
        self._sfd = None
        self.procs: List[Worker] = []
        self.main_loop_interrupt = threading.Event()
        self.interrupt_signal = False
        self.interrupt_child = None

    def build_ssl_context(self, cert: Optional[Path], key: Optional[Path]):
        if not (cert and key):
            self.ssl_ctx = (False, None, None)
            return
        ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        ctx.load_cert_chain(cert, key, None)
        self.ssl_enabled = True
        # with cert.open("rb") as f:
        #     cert_contents = f.read()
        # with key.open("rb") as f:
        #     key_contents = f.read()
        self.ssl_ctx = (True, str(cert.resolve()), str(key.resolve()))

    @staticmethod
    def _spawn_asgi_worker(
        worker_id,
        callback_loader,
        socket,
        loop_impl,
        threads,
        pthreads,
        threading_mode,
        http_mode,
        http1_buffer_size,
        websockets,
        loop_opt,
        log_enabled,
        log_level,
        log_config,
        ssl_ctx,
        scope_opts,
    ):
        from granian._loops import loops, set_loop_signals

        configure_logging(log_level, log_config, log_enabled)

        loop = loops.get(loop_impl)
        sfd = socket.fileno()
        callback = callback_loader()
        lifespan_handler = LifespanProtocol(callback)

        loop.run_until_complete(lifespan_handler.startup())
        if lifespan_handler.interrupt:
            logger.error('ASGI lifespan startup failed')
            sys.exit(1)

        shutdown_event = set_loop_signals(loop, [signal.SIGTERM, signal.SIGINT])

        wcallback = _asgi_call_wrap(callback, scope_opts)
        if not loop_opt:
            wcallback = future_watcher_wrapper(wcallback)

        worker = ASGIWorker(
            worker_id, sfd, threads, pthreads, http_mode, http1_buffer_size, websockets, loop_opt, *ssl_ctx
        )
        serve = getattr(worker, {ThreadModes.runtime: 'serve_rth', ThreadModes.workers: 'serve_wth'}[threading_mode])
        serve(wcallback, loop, contextvars.copy_context(), shutdown_event.wait())
        loop.run_until_complete(lifespan_handler.shutdown())

    @staticmethod
    def _spawn_rsgi_worker(
        worker_id,
        callback_loader,
        socket,
        loop_impl,
        threads,
        pthreads,
        threading_mode,
        http_mode,
        http1_buffer_size,
        websockets,
        loop_opt,
        log_enabled,
        log_level,
        log_config,
        ssl_ctx,
        scope_opts,
    ):
        from granian._loops import loops, set_loop_signals

        configure_logging(log_level, log_config, log_enabled)

        loop = loops.get(loop_impl)
        sfd = socket.fileno()
        target = callback_loader()
        callback = getattr(target, '__rsgi__') if hasattr(target, '__rsgi__') else target
        callback_init = (
            getattr(target, '__rsgi_init__') if hasattr(target, '__rsgi_init__') else lambda *args, **kwargs: None
        )

        shutdown_event = set_loop_signals(loop, [signal.SIGTERM, signal.SIGINT])
        callback_init(loop)

        worker = RSGIWorker(
            worker_id, sfd, threads, pthreads, http_mode, http1_buffer_size, websockets, loop_opt, *ssl_ctx
        )
        serve = getattr(worker, {ThreadModes.runtime: 'serve_rth', ThreadModes.workers: 'serve_wth'}[threading_mode])
        serve(
            future_watcher_wrapper(callback) if not loop_opt else callback,
            loop,
            contextvars.copy_context(),
            shutdown_event.wait(),
        )

    @staticmethod
    def _spawn_wsgi_worker(
        worker_id,
        callback_loader,
        socket,
        loop_impl,
        threads,
        pthreads,
        threading_mode,
        http_mode,
        http1_buffer_size,
        websockets,
        loop_opt,
        log_enabled,
        log_level,
        log_config,
        ssl_ctx,
        scope_opts,
    ):
        from granian._loops import loops, set_loop_signals

        configure_logging(log_level, log_config, log_enabled)

        loop = loops.get(loop_impl)
        sfd = socket.fileno()
        callback = callback_loader()

        shutdown_event = set_loop_signals(loop, [signal.SIGTERM, signal.SIGINT])

        worker = WSGIWorker(worker_id, sfd, threads, pthreads, http_mode, http1_buffer_size, *ssl_ctx)
        serve = getattr(worker, {ThreadModes.runtime: 'serve_rth', ThreadModes.workers: 'serve_wth'}[threading_mode])
        serve(_wsgi_call_wrap(callback, scope_opts), loop, contextvars.copy_context(), shutdown_event.wait())

    def _init_shared_socket(self):
        self._shd = SocketHolder.from_address(self.bind_addr, self.bind_port, self.backlog)
        self._sfd = self._shd.get_fd()

    def signal_handler(self, *args, **kwargs):
        self.interrupt_signal = True
        self.main_loop_interrupt.set()

    def _spawn_proc(self, idx, target, callback_loader, socket_loader) -> Worker:
        return Worker(
            parent=self,
            idx=idx,
            target=target,
            args=(
                idx + 1,
                callback_loader,
                socket_loader(),
                self.loop,
                self.threads,
                self.pthreads,
                self.threading_mode,
                self.http,
                self.http1_buffer_size,
                self.websockets,
                self.loop_opt,
                self.log_enabled,
                self.log_level,
                self.log_config,
                self.ssl_ctx,
                {'url_path_prefix': self.url_path_prefix},
            ),
        )

    def _spawn_workers(self, sock, spawn_target, target_loader):
        def socket_loader():
            return sock

        for idx in range(self.workers):
            proc = self._spawn_proc(
                idx=idx, target=spawn_target, callback_loader=target_loader, socket_loader=socket_loader
            )
            proc.start()
            self.procs.append(proc)

    def _stop_workers(self):
        for proc in self.procs:
            proc.terminate()
        for proc in self.procs:
            proc.join()
        self.procs.clear()

    def startup(self, spawn_target, target_loader):
        logger.info('Starting granian')

        for sig in self.SIGNALS:
            signal.signal(sig, self.signal_handler)

        self._init_shared_socket()
        sock = socket.socket(fileno=self._sfd)
        sock.set_inheritable(True)
        logger.info(f'Listening at: {self.bind_addr}:{self.bind_port}')

        self._spawn_workers(sock, spawn_target, target_loader)
        return sock

    def shutdown(self, exit_code=0):
        logger.info('Shutting down granian')
        self._stop_workers()
        if not exit_code and self.interrupt_child is not None:
            exit_code = 1
        if exit_code:
            sys.exit(exit_code)

    def _serve_loop(self):
        while True:
            self.main_loop_interrupt.wait()
            if self.interrupt_signal:
                break
            if self.interrupt_child is not None:
                break

    def _serve(self, spawn_target, target_loader):
        self.startup(spawn_target, target_loader)
        self._serve_loop()
        self.shutdown()

    def _serve_with_reloader(self, spawn_target, target_loader):
        reload_path = Path.cwd()
        sock = self.startup(spawn_target, target_loader)

        try:
            for _ in watchfiles.watch(reload_path, stop_event=self.main_loop_interrupt):
                logger.info('Changes detected, reloading workers..')
                self._stop_workers()
                self._spawn_workers(sock, spawn_target, target_loader)
        except StopIteration:
            pass

        self.shutdown()

    def serve(
        self,
        spawn_target: Optional[Callable[..., None]] = None,
        target_loader: Optional[Callable[..., Callable[..., Any]]] = None,
        wrap_loader: bool = True,
    ):
        default_spawners = {
            Interfaces.ASGI: self._spawn_asgi_worker,
            Interfaces.RSGI: self._spawn_rsgi_worker,
            Interfaces.WSGI: self._spawn_wsgi_worker,
        }
        if target_loader:
            if wrap_loader:
                target_loader = partial(target_loader, self.target)
        else:
            target_loader = partial(load_target, self.target)

        if not spawn_target:
            spawn_target = default_spawners[self.interface]
            if sys.platform == 'win32' and self.workers > 1:
                self.workers = 1
                logger.warn(
                    "Due to a bug in Windows unblocking socket implementation "
                    "granian can't support multiple workers on this platform. "
                    "Number of workers will now fallback to 1."
                )

        serve_method = self._serve_with_reloader if self.reload_on_changes else self._serve
        serve_method(spawn_target, target_loader)
