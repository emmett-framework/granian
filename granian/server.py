import contextvars
import multiprocessing
import signal
import socket
import ssl
import threading

from functools import partial
from pathlib import Path
from typing import List, Optional

from ._granian import ASGIWorker, RSGIWorker, WSGIWorker
from ._internal import load_target
from .asgi import LifespanProtocol, _callback_wrapper as _asgi_call_wrap
from .constants import Interfaces, HTTPModes, Loops, ThreadModes
from .log import LogLevels, configure_logging, logger
from .net import SocketHolder
from .rsgi import _callback_wrapper as _rsgi_call_wrap
from .wsgi import _callback_wrapper as _wsgi_call_wrap

multiprocessing.allow_connection_pickling()


class Granian:
    SIGNALS = {signal.SIGINT, signal.SIGTERM}

    def __init__(
        self,
        target: str,
        address: str = "127.0.0.1",
        port: int = 8000,
        interface: Interfaces = Interfaces.RSGI,
        workers: int = 1,
        threads: int = 1,
        pthreads: int = 1,
        threading_mode: ThreadModes = ThreadModes.workers,
        loop: Loops = Loops.auto,
        http: HTTPModes = HTTPModes.auto,
        websockets: bool = True,
        backlog: int = 1024,
        http1_buffer_size: int = 65535,
        log_level: LogLevels = LogLevels.info,
        ssl_cert: Optional[Path] = None,
        ssl_key: Optional[Path] = None,
        url_path_prefix: Optional[str] = None
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
        self.http = http
        self.websockets = websockets
        self.backlog = max(128, backlog)
        self.http1_buffer_size = http1_buffer_size
        self.log_level = log_level
        self.url_path_prefix = url_path_prefix
        configure_logging(self.log_level)
        self.build_ssl_context(ssl_cert, ssl_key)
        self._shd = None
        self._sfd = None
        self.procs: List[multiprocessing.Process] = []
        self.exit_event = threading.Event()

    def build_ssl_context(
        self,
        cert: Optional[Path],
        key: Optional[Path]
    ):
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
        log_level,
        ssl_ctx,
        scope_opts
    ):
        from granian._loops import loops, set_loop_signals

        configure_logging(log_level)
        loop = loops.get(loop_impl)
        sfd = socket.fileno()
        callback = callback_loader()
        lifespan_handler = LifespanProtocol(callback)

        loop.run_until_complete(lifespan_handler.startup())
        if lifespan_handler.interrupt:
            return

        shutdown_event = set_loop_signals(loop, [signal.SIGTERM, signal.SIGINT])

        worker = ASGIWorker(
            worker_id,
            sfd,
            threads,
            pthreads,
            http_mode,
            http1_buffer_size,
            websockets,
            *ssl_ctx
        )
        serve = getattr(worker, {
            ThreadModes.runtime: "serve_rth",
            ThreadModes.workers: "serve_wth"
        }[threading_mode])
        serve(
            _asgi_call_wrap(callback, scope_opts),
            loop,
            contextvars.copy_context(),
            shutdown_event.wait()
        )
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
        log_level,
        ssl_ctx,
        scope_opts
    ):
        from granian._loops import loops, set_loop_signals

        configure_logging(log_level)
        loop = loops.get(loop_impl)
        sfd = socket.fileno()
        target = callback_loader()
        callback = (
            getattr(target, '__rsgi__') if hasattr(target, '__rsgi__') else
            target
        )
        callback_init = (
            getattr(target, '__rsgi_init__') if hasattr(target, '__rsgi_init__') else
            lambda *args, **kwargs: None
        )

        shutdown_event = set_loop_signals(loop, [signal.SIGTERM, signal.SIGINT])
        callback_init(loop)

        worker = RSGIWorker(
            worker_id,
            sfd,
            threads,
            pthreads,
            http_mode,
            http1_buffer_size,
            websockets,
            *ssl_ctx
        )
        serve = getattr(worker, {
            ThreadModes.runtime: "serve_rth",
            ThreadModes.workers: "serve_wth"
        }[threading_mode])
        serve(
            _rsgi_call_wrap(callback),
            loop,
            contextvars.copy_context(),
            shutdown_event.wait()
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
        log_level,
        ssl_ctx,
        scope_opts
    ):
        from granian._loops import loops, set_loop_signals

        configure_logging(log_level)
        loop = loops.get(loop_impl)
        sfd = socket.fileno()
        callback = callback_loader()

        shutdown_event = set_loop_signals(loop, [signal.SIGTERM, signal.SIGINT])

        worker = WSGIWorker(
            worker_id,
            sfd,
            threads,
            pthreads,
            http_mode,
            http1_buffer_size,
            *ssl_ctx
        )
        serve = getattr(worker, {
            ThreadModes.runtime: "serve_rth",
            ThreadModes.workers: "serve_wth"
        }[threading_mode])
        serve(
            _wsgi_call_wrap(callback, scope_opts),
            loop,
            contextvars.copy_context(),
            shutdown_event.wait()
        )

    def _init_shared_socket(self):
        self._shd = SocketHolder.from_address(
            self.bind_addr,
            self.bind_port,
            self.backlog
        )
        self._sfd = self._shd.get_fd()

    def signal_handler(self, *args, **kwargs):
        self.exit_event.set()

    def _spawn_proc(
        self,
        id,
        target,
        callback_loader,
        socket_loader
    ) -> multiprocessing.Process:
        return multiprocessing.get_context().Process(
            name="granian-worker",
            target=target,
            args=(
                id,
                callback_loader,
                socket_loader(),
                self.loop,
                self.threads,
                self.pthreads,
                self.threading_mode,
                self.http,
                self.http1_buffer_size,
                self.websockets,
                self.log_level,
                self.ssl_ctx,
                {
                    "url_path_prefix": self.url_path_prefix
                }
            )
        )

    def startup(self, spawn_target, target_loader):
        logger.info("Starting granian")

        for sig in self.SIGNALS:
            signal.signal(sig, self.signal_handler)

        self._init_shared_socket()
        sock = socket.socket(fileno=self._sfd)
        sock.set_inheritable(True)
        logger.info(f"Listening at: {self.bind_addr}:{self.bind_port}")

        def socket_loader():
            return sock

        for idx in range(self.workers):
            proc = self._spawn_proc(
                id=idx + 1,
                target=spawn_target,
                callback_loader=target_loader,
                socket_loader=socket_loader
            )
            proc.start()
            self.procs.append(proc)
            logger.info(f"Booting worker-{idx + 1} with pid: {proc.pid}")

    def shutdown(self):
        logger.info("Shutting down granian")
        for proc in self.procs:
            proc.terminate()
        for proc in self.procs:
            proc.join()

    def serve(self, spawn_target = None, target_loader = None):
        default_spawners = {
            Interfaces.ASGI: self._spawn_asgi_worker,
            Interfaces.RSGI: self._spawn_rsgi_worker,
            Interfaces.WSGI: self._spawn_wsgi_worker
        }
        target_loader = target_loader or load_target
        spawn_target = spawn_target or default_spawners[self.interface]

        self.startup(spawn_target, partial(target_loader, self.target))
        self.exit_event.wait()
        self.shutdown()
