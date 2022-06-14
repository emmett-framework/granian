import contextvars
import copyreg
import os
import multiprocessing
import signal
import socket
import threading

from functools import partial
from typing import List, Optional

from . import _workers
from ._internal import CTX, load_target
from .asgi import callback_wrapper as _asgi_call_wrap
from .constants import Interfaces, ThreadModes
from .net import SocketHolder
from .rsgi import callback_wrapper as _rsgi_call_wrap

multiprocessing.allow_connection_pickling()

ASGIWorker = _workers.ASGIWorker
RSGIWorker = _workers.RSGIWorker


class Granian:
    SIGNALS = {signal.SIGINT, signal.SIGTERM}

    def __init__(
        self,
        target: str,
        address: str = "127.0.0.1",
        port: int = 8000,
        workers: int = 1,
        backlog: int = 1024,
        threads: Optional[int] = None,
        threading_mode: ThreadModes = ThreadModes.runtime,
        http1_buffer_size: int = 65535,
        interface: Interfaces = Interfaces.RSGI
    ):
        self.target = target
        self.bind_addr = address
        self.bind_port = port
        self.workers = max(1, workers)
        self.backlog = max(128, backlog)
        self.threads = (
            max(1, threads) if threads is not None else
            max(2, multiprocessing.cpu_count() // workers)
        )
        self.threading_mode = threading_mode
        self.http1_buffer_size = http1_buffer_size
        self.interface = interface
        self._sfd = None
        self.procs: List[multiprocessing.Process] = []
        self.exit_event = threading.Event()

    @staticmethod
    def _target_load(target: str):
        return load_target(target)

    @staticmethod
    def _spawn_asgi_worker(
        worker_id,
        callback_loader,
        socket,
        threads,
        threading_mode,
        http1_buffer_size
    ):
        from granian._loops import loops

        loop = loops.get("auto")
        sfd = socket.fileno()
        callback = callback_loader()

        worker = ASGIWorker(worker_id, sfd, threads, http1_buffer_size)
        serve = getattr(worker, {
            ThreadModes.runtime: "serve_rth",
            ThreadModes.workers: "serve_wth"
        }[threading_mode])
        serve(_asgi_call_wrap(callback), loop, contextvars.copy_context())

        # worker._serve_ret_st(callback, loop, contextvars.copy_context())
        # print("infinite loop")
        # loop.run_forever()

    @staticmethod
    def _spawn_rsgi_worker(
        worker_id,
        callback_loader,
        socket,
        threads,
        threading_mode,
        http1_buffer_size
    ):
        from granian._loops import loops

        loop = loops.get("auto")
        sfd = socket.fileno()
        callback = callback_loader()

        worker = RSGIWorker(worker_id, sfd, threads, http1_buffer_size)
        serve = getattr(worker, {
            ThreadModes.runtime: "serve_rth",
            ThreadModes.workers: "serve_wth"
        }[threading_mode])
        serve(_rsgi_call_wrap(callback), loop, contextvars.copy_context())

        # worker._serve_ret_st(callback, loop, contextvars.copy_context())
        # print("infinite loop")
        # loop.run_forever()

    @staticmethod
    def _shared_socket_loader(pid):
        return CTX.socks[pid]

    @staticmethod
    def _local_socket_builder(addr, port, backlog):
        return SocketHolder.from_address(addr, port, backlog)

    def _init_shared_socket(self, pid):
        # if self.workers > 1:
        CTX.socks[pid] = SocketHolder.from_address(
            self.bind_addr,
            self.bind_port,
            self.backlog
        )
        self._sfd = CTX.socks[pid].get_fd()

    def _build_socket_loader(self, pid):
        if self.workers > 1:
            return partial(self._shared_socket_loader, pid)
        return partial(
            self._local_socket_builder,
            self.bind_addr,
            self.bind_port,
            self.backlog
        )

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
            target=target,
            args=(
                id,
                callback_loader,
                socket_loader(),
                self.threads,
                self.threading_mode,
                self.http1_buffer_size
            )
        )

    def startup(self, spawn_target, target_loader):
        for sig in self.SIGNALS:
            signal.signal(sig, self.signal_handler)

        pid = os.getpid()
        self._init_shared_socket(pid)

        sock = socket.socket(fileno=self._sfd)
        sock.set_inheritable(True)

        def socket_loader():
            return sock

        for idx in range(self.workers):
            proc = self._spawn_proc(
                id=idx,
                target=spawn_target,
                callback_loader=target_loader,
                socket_loader=socket_loader
            )
            proc.start()
            self.procs.append(proc)

    def shutdown(self):
        print("send term")
        for proc in self.procs:
            # proc.terminate()
            proc.kill()
        print("joining")
        for proc in self.procs:
            proc.join()

    def serve(self, spawn_target = None, target_loader = None):
        default_spawners = {
            Interfaces.ASGI: self._spawn_asgi_worker,
            Interfaces.RSGI: self._spawn_rsgi_worker
        }
        target_loader = target_loader or self._target_load
        spawn_target = spawn_target or default_spawners[self.interface]

        # if self.workers > 1 and "fork" not in multiprocessing.get_all_start_methods():
        #     raise RuntimeError("Multiple workers are not supported on current platform")

        self.startup(spawn_target, partial(target_loader, self.target))
        print("started", self.procs)
        try:
            self.exit_event.wait()
        except KeyboardInterrupt:
            print("keyb. interr")
        print("exit event received")
        self.shutdown()


copyreg.pickle(
    SocketHolder,
    lambda v: (SocketHolder, v.__getstate__())
)
