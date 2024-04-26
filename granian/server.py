from __future__ import annotations

import contextvars
import multiprocessing
import os
import signal
import socket
import ssl
import sys
import threading
import time
from functools import partial
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

from ._futures import future_watcher_wrapper
from ._granian import ASGIWorker, RSGIWorker, WSGIWorker
from ._imports import setproctitle, watchfiles
from ._internal import load_target
from .asgi import LifespanProtocol, _callback_wrapper as _asgi_call_wrap
from .constants import HTTPModes, Interfaces, Loops, ThreadModes
from .http import HTTP1Settings, HTTP2Settings
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
            self.parent.interrupt_children.append(self.idx)
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
        http1_settings: Optional[HTTP1Settings] = None,
        http2_settings: Optional[HTTP2Settings] = None,
        log_enabled: bool = True,
        log_level: LogLevels = LogLevels.info,
        log_dictconfig: Optional[Dict[str, Any]] = None,
        ssl_cert: Optional[Path] = None,
        ssl_key: Optional[Path] = None,
        url_path_prefix: Optional[str] = None,
        respawn_failed_workers: bool = False,
        respawn_interval: float = 3.5,
        reload: bool = False,
        process_name: Optional[str] = None,
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
        self.http1_settings = http1_settings
        self.http2_settings = http2_settings
        self.log_enabled = log_enabled
        self.log_level = log_level
        self.log_config = log_dictconfig
        self.url_path_prefix = url_path_prefix
        self.respawn_failed_workers = respawn_failed_workers
        self.reload_on_changes = reload
        self.respawn_interval = respawn_interval
        self.process_name = process_name

        configure_logging(self.log_level, self.log_config, self.log_enabled)

        self.build_ssl_context(ssl_cert, ssl_key)
        self._shd = None
        self._sfd = None
        self.procs: List[Worker] = []
        self.main_loop_interrupt = threading.Event()
        self.interrupt_signal = False
        self.interrupt_children = []
        self.respawned_procs = {}
        self.reload_signal = False

    def build_ssl_context(self, cert: Optional[Path], key: Optional[Path]):
        if not (cert and key):
            self.ssl_ctx = (False, None, None)
            return
        ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        ctx.load_cert_chain(cert, key, None)
        # with cert.open("rb") as f:
        #     cert_contents = f.read()
        # with key.open("rb") as f:
        #     key_contents = f.read()
        self.ssl_ctx = (True, str(cert.resolve()), str(key.resolve()))

    @staticmethod
    def _spawn_asgi_worker(
        worker_id: int,
        process_name: Optional[str],
        callback_loader: Callable[..., Any],
        socket: socket.socket,
        loop_impl: Loops,
        threads: int,
        pthreads: int,
        threading_mode: ThreadModes,
        http_mode: HTTPModes,
        http1_settings: Optional[HTTP1Settings],
        http2_settings: Optional[HTTP2Settings],
        websockets: bool,
        loop_opt: bool,
        log_enabled: bool,
        log_level: LogLevels,
        log_config: Dict[str, Any],
        ssl_ctx: Tuple[bool, Optional[str], Optional[str]],
        scope_opts: Dict[str, Any],
    ):
        from granian._loops import loops, set_loop_signals

        if process_name:
            setproctitle.setproctitle(f'{process_name} worker-{worker_id}')
        configure_logging(log_level, log_config, log_enabled)

        loop = loops.get(loop_impl)
        sfd = socket.fileno()
        callback = callback_loader()

        shutdown_event = set_loop_signals(loop, [signal.SIGTERM, signal.SIGINT])

        wcallback = _asgi_call_wrap(callback, scope_opts, {})
        if not loop_opt:
            wcallback = future_watcher_wrapper(wcallback)

        worker = ASGIWorker(
            worker_id, sfd, threads, pthreads, http_mode, http1_settings, http2_settings, websockets, loop_opt, *ssl_ctx
        )
        serve = getattr(worker, {ThreadModes.runtime: 'serve_rth', ThreadModes.workers: 'serve_wth'}[threading_mode])
        serve(wcallback, loop, contextvars.copy_context(), shutdown_event)

    @staticmethod
    def _spawn_asgi_lifespan_worker(
        worker_id: int,
        process_name: Optional[str],
        callback_loader: Callable[..., Any],
        socket: socket.socket,
        loop_impl: Loops,
        threads: int,
        pthreads: int,
        threading_mode: ThreadModes,
        http_mode: HTTPModes,
        http1_settings: Optional[HTTP1Settings],
        http2_settings: Optional[HTTP2Settings],
        websockets: bool,
        loop_opt: bool,
        log_enabled: bool,
        log_level: LogLevels,
        log_config: Dict[str, Any],
        ssl_ctx: Tuple[bool, Optional[str], Optional[str]],
        scope_opts: Dict[str, Any],
    ):
        from granian._loops import loops, set_loop_signals

        if process_name:
            setproctitle.setproctitle(f'{process_name} worker-{worker_id}')
        configure_logging(log_level, log_config, log_enabled)

        loop = loops.get(loop_impl)
        sfd = socket.fileno()
        callback = callback_loader()
        lifespan_handler = LifespanProtocol(callback)

        loop.run_until_complete(lifespan_handler.startup())
        if lifespan_handler.interrupt:
            logger.error('ASGI lifespan startup failed', exc_info=lifespan_handler.exc)
            sys.exit(1)

        shutdown_event = set_loop_signals(loop, [signal.SIGTERM, signal.SIGINT])

        wcallback = _asgi_call_wrap(callback, scope_opts, lifespan_handler.state)
        if not loop_opt:
            wcallback = future_watcher_wrapper(wcallback)

        worker = ASGIWorker(
            worker_id, sfd, threads, pthreads, http_mode, http1_settings, http2_settings, websockets, loop_opt, *ssl_ctx
        )
        serve = getattr(worker, {ThreadModes.runtime: 'serve_rth', ThreadModes.workers: 'serve_wth'}[threading_mode])
        serve(wcallback, loop, contextvars.copy_context(), shutdown_event)
        loop.run_until_complete(lifespan_handler.shutdown())

    @staticmethod
    def _spawn_rsgi_worker(
        worker_id: int,
        process_name: Optional[str],
        callback_loader: Callable[..., Any],
        socket: socket.socket,
        loop_impl: Loops,
        threads: int,
        pthreads: int,
        threading_mode: ThreadModes,
        http_mode: HTTPModes,
        http1_settings: Optional[HTTP1Settings],
        http2_settings: Optional[HTTP2Settings],
        websockets: bool,
        loop_opt: bool,
        log_enabled: bool,
        log_level: LogLevels,
        log_config: Dict[str, Any],
        ssl_ctx: Tuple[bool, Optional[str], Optional[str]],
        scope_opts: Dict[str, Any],
    ):
        from granian._loops import loops, set_loop_signals

        if process_name:
            setproctitle.setproctitle(f'{process_name} worker-{worker_id}')
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
            worker_id, sfd, threads, pthreads, http_mode, http1_settings, http2_settings, websockets, loop_opt, *ssl_ctx
        )
        serve = getattr(worker, {ThreadModes.runtime: 'serve_rth', ThreadModes.workers: 'serve_wth'}[threading_mode])
        serve(
            future_watcher_wrapper(callback) if not loop_opt else callback,
            loop,
            contextvars.copy_context(),
            shutdown_event,
        )

    @staticmethod
    def _spawn_wsgi_worker(
        worker_id: int,
        process_name: Optional[str],
        callback_loader: Callable[..., Any],
        socket: socket.socket,
        loop_impl: Loops,
        threads: int,
        pthreads: int,
        threading_mode: ThreadModes,
        http_mode: HTTPModes,
        http1_settings: Optional[HTTP1Settings],
        http2_settings: Optional[HTTP2Settings],
        websockets: bool,
        loop_opt: bool,
        log_enabled: bool,
        log_level: LogLevels,
        log_config: Dict[str, Any],
        ssl_ctx: Tuple[bool, Optional[str], Optional[str]],
        scope_opts: Dict[str, Any],
    ):
        from granian._loops import loops, set_loop_signals

        if process_name:
            setproctitle.setproctitle(f'{process_name} worker-{worker_id}')
        configure_logging(log_level, log_config, log_enabled)

        loop = loops.get(loop_impl)
        sfd = socket.fileno()
        callback = callback_loader()

        shutdown_event = set_loop_signals(loop, [signal.SIGTERM, signal.SIGINT])

        worker = WSGIWorker(worker_id, sfd, threads, pthreads, http_mode, http1_settings, http2_settings, *ssl_ctx)
        serve = getattr(worker, {ThreadModes.runtime: 'serve_rth', ThreadModes.workers: 'serve_wth'}[threading_mode])
        serve(_wsgi_call_wrap(callback, scope_opts), loop, contextvars.copy_context(), shutdown_event)

    def _init_shared_socket(self):
        self._shd = SocketHolder.from_address(self.bind_addr, self.bind_port, self.backlog)
        self._sfd = self._shd.get_fd()

    def signal_handler_interrupt(self, *args, **kwargs):
        self.interrupt_signal = True
        self.main_loop_interrupt.set()

    def signal_handler_reload(self, *args, **kwargs):
        self.reload_signal = True
        self.main_loop_interrupt.set()

    def _spawn_proc(self, idx, target, callback_loader, socket_loader) -> Worker:
        return Worker(
            parent=self,
            idx=idx,
            target=target,
            args=(
                idx + 1,
                self.process_name,
                callback_loader,
                socket_loader(),
                self.loop,
                self.threads,
                self.pthreads,
                self.threading_mode,
                self.http,
                self.http1_settings,
                self.http2_settings,
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

    def _respawn_workers(self, workers, sock, spawn_target, target_loader, delay: float = 0):
        def socket_loader():
            return sock

        for idx in workers:
            self.respawned_procs[idx] = time.time()
            logger.info(f'Respawning worker-{idx + 1}')
            old_proc = self.procs.pop(idx)
            proc = self._spawn_proc(
                idx=idx, target=spawn_target, callback_loader=target_loader, socket_loader=socket_loader
            )
            proc.start()
            self.procs.insert(idx, proc)
            time.sleep(delay)
            logger.info(f'Stopping old worker-{idx + 1}')
            old_proc.terminate()
            old_proc.join()

    def _stop_workers(self):
        for proc in self.procs:
            proc.terminate()
        for proc in self.procs:
            proc.join()
        self.procs.clear()

    def setup_signals(self):
        signals = [signal.SIGINT, signal.SIGTERM]
        if sys.platform == 'win32':
            signals.append(signal.SIGBREAK)

        for sig in signals:
            signal.signal(sig, self.signal_handler_interrupt)

        if sys.platform != 'win32':
            signal.signal(signal.SIGHUP, self.signal_handler_reload)

    def startup(self, spawn_target, target_loader):
        logger.info(f'Starting granian (main PID: {os.getpid()})')

        self.setup_signals()
        self._init_shared_socket()
        sock = socket.socket(fileno=self._sfd)
        sock.set_inheritable(True)
        proto = 'https' if self.ssl_ctx[0] else 'http'
        logger.info(f'Listening at: {proto}://{self.bind_addr}:{self.bind_port}')

        self._spawn_workers(sock, spawn_target, target_loader)
        return sock

    def shutdown(self, exit_code=0):
        logger.info('Shutting down granian')
        self._stop_workers()
        if not exit_code and self.interrupt_children:
            exit_code = 1
        if exit_code:
            sys.exit(exit_code)

    def _serve_loop(self, sock, spawn_target, target_loader):
        while True:
            self.main_loop_interrupt.wait()
            if self.interrupt_signal:
                break

            if self.interrupt_children:
                if not self.respawn_failed_workers:
                    break

                cycle = time.time()
                if any(cycle - self.respawned_procs.get(idx, 0) <= 5.5 for idx in self.interrupt_children):
                    logger.error('Worker crash loop detected, exiting')
                    break

                workers = list(self.interrupt_children)
                self.interrupt_children.clear()
                self.respawned_procs.clear()
                self.main_loop_interrupt.clear()
                self._respawn_workers(workers, sock, spawn_target, target_loader)

            if self.reload_signal:
                logger.info('HUP signal received, gracefully respawning workers..')
                workers = list(range(self.workers))
                self.reload_signal = False
                self.respawned_procs.clear()
                self.main_loop_interrupt.clear()
                self._respawn_workers(workers, sock, spawn_target, target_loader, delay=self.respawn_interval)

    def _serve(self, spawn_target, target_loader):
        sock = self.startup(spawn_target, target_loader)
        self._serve_loop(sock, spawn_target, target_loader)
        self.shutdown()

    def _serve_with_reloader(self, spawn_target, target_loader):
        if watchfiles is None:
            logger.error('Using --reload requires the granian[reload] extra')
            sys.exit(1)

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
            Interfaces.ASGI: self._spawn_asgi_lifespan_worker,
            Interfaces.ASGINL: self._spawn_asgi_worker,
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
                    'Due to a bug in Windows unblocking socket implementation '
                    "granian can't support multiple workers on this platform. "
                    'Number of workers will now fallback to 1.'
                )

        if self.websockets:
            if self.interface == Interfaces.WSGI:
                logger.info('Websockets are not supported on WSGI')
            if self.http == HTTPModes.http2:
                logger.info('Websockets are not supported on HTTP/2 only')

        if setproctitle is not None:
            self.process_name = self.process_name or (
                f'granian {self.interface} {self.bind_addr}:{self.bind_port} {self.target}'
            )
            setproctitle.setproctitle(self.process_name)
        elif self.process_name is not None:
            logger.error('Setting process name requires the granian[pname] extra')
            sys.exit(1)

        serve_method = self._serve_with_reloader if self.reload_on_changes else self._serve
        serve_method(spawn_target, target_loader)
