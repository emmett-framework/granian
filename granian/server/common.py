from __future__ import annotations

import errno
import multiprocessing
import os
import ssl
import sys
import threading
import time
from functools import partial
from pathlib import Path
from typing import Any, Callable, Dict, Generic, List, Optional, Sequence, Type, TypeVar

from .._compat import _PY_312, _PYV
from .._imports import setproctitle, watchfiles
from .._internal import load_target
from .._signals import set_main_signals
from ..constants import HTTPModes, Interfaces, Loops, RuntimeModes, TaskImpl
from ..errors import ConfigurationError, PidFileError
from ..http import HTTP1Settings, HTTP2Settings
from ..log import DEFAULT_ACCESSLOG_FMT, LogLevels, configure_logging, logger
from ..net import SocketHolder


WT = TypeVar('WT')


class AbstractWorker:
    _idl = 'id'

    def __init__(self, parent: AbstractServer, idx: int, target: Any, args: Any):
        self.parent = parent
        self.idx = idx
        self.interrupt_by_parent = False
        self.birth = time.time()
        self._spawn(target, args)

    def _spawn(self, target, args):
        raise NotImplementedError

    def _id(self):
        raise NotImplementedError

    def _watcher(self):
        self.inner.join()
        if not self.interrupt_by_parent:
            logger.error(f'Unexpected exit from worker-{self.idx + 1}')
            self.parent.interrupt_children.append(self.idx)
            self.parent.main_loop_interrupt.set()

    def _watch(self):
        watcher = threading.Thread(target=self._watcher)
        watcher.start()

    def start(self):
        self.inner.start()
        logger.info(f'Spawning worker-{self.idx + 1} with {self._idl}: {self._id()}')
        self._watch()

    def is_alive(self):
        return self.inner.is_alive()

    def terminate(self):
        raise NotImplementedError

    def kill(self):
        raise NotImplementedError

    def join(self, timeout=None):
        self.inner.join(timeout=timeout)


class AbstractServer(Generic[WT]):
    def __init__(
        self,
        target: str,
        address: str = '127.0.0.1',
        port: int = 8000,
        interface: Interfaces = Interfaces.RSGI,
        workers: int = 1,
        blocking_threads: Optional[int] = None,
        blocking_threads_idle_timeout: int = 30,
        runtime_threads: int = 1,
        runtime_blocking_threads: Optional[int] = None,
        runtime_mode: RuntimeModes = RuntimeModes.st,
        loop: Loops = Loops.auto,
        task_impl: TaskImpl = TaskImpl.asyncio,
        http: HTTPModes = HTTPModes.auto,
        websockets: bool = True,
        backlog: int = 1024,
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
        url_path_prefix: Optional[str] = None,
        respawn_failed_workers: bool = False,
        respawn_interval: float = 3.5,
        workers_lifetime: Optional[int] = None,
        workers_kill_timeout: Optional[int] = None,
        factory: bool = False,
        reload: bool = False,
        reload_paths: Optional[Sequence[Path]] = None,
        reload_ignore_dirs: Optional[Sequence[str]] = None,
        reload_ignore_patterns: Optional[Sequence[str]] = None,
        reload_ignore_paths: Optional[Sequence[Path]] = None,
        reload_filter: Optional[Type[watchfiles.BaseFilter]] = None,
        process_name: Optional[str] = None,
        pid_file: Optional[Path] = None,
    ):
        self.target = target
        self.bind_addr = address
        self.bind_port = port
        self.interface = interface
        self.workers = max(1, workers)
        self.runtime_threads = max(1, runtime_threads)
        self.runtime_blocking_threads = 512 if runtime_blocking_threads is None else max(1, runtime_blocking_threads)
        self.runtime_mode = runtime_mode
        self.loop = loop
        self.task_impl = task_impl
        self.http = http
        self.websockets = websockets
        self.backlog = max(128, backlog)
        self.backpressure = max(1, backpressure or self.backlog // self.workers)
        self.blocking_threads = (
            blocking_threads
            if blocking_threads is not None
            else max(1, (self.backpressure // 2) if self.interface == Interfaces.WSGI else 1)
        )
        self.blocking_threads_idle_timeout = blocking_threads_idle_timeout
        self.http1_settings = http1_settings
        self.http2_settings = http2_settings
        self.log_enabled = log_enabled
        self.log_level = log_level
        self.log_config = log_dictconfig
        self.log_access = log_access
        self.log_access_format = log_access_format or DEFAULT_ACCESSLOG_FMT
        self.url_path_prefix = url_path_prefix
        self.respawn_failed_workers = respawn_failed_workers
        self.reload_on_changes = reload
        self.respawn_interval = respawn_interval
        self.workers_lifetime = workers_lifetime
        self.workers_kill_timeout = workers_kill_timeout
        self.factory = factory
        self.reload_paths = reload_paths or [Path.cwd()]
        self.reload_ignore_paths = reload_ignore_paths or ()
        self.reload_ignore_dirs = reload_ignore_dirs or ()
        self.reload_ignore_patterns = reload_ignore_patterns or ()
        self.reload_filter = reload_filter
        self.process_name = process_name
        self.pid_file = pid_file

        configure_logging(self.log_level, self.log_config, self.log_enabled)

        self.build_ssl_context(ssl_cert, ssl_key, ssl_key_password)
        self._shd = None
        self._sfd = None
        self._sso = None
        self.wrks: List[WT] = []
        self.main_loop_interrupt = threading.Event()
        self.interrupt_signal = False
        self.interrupt_children = []
        self.respawned_wrks = {}
        self.reload_signal = False
        self.lifetime_signal = False
        self.pid = None

    def build_ssl_context(self, cert: Optional[Path], key: Optional[Path], password: Optional[str]):
        if not (cert and key):
            self.ssl_ctx = (False, None, None)
            return
        ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        ctx.load_cert_chain(cert, key, password)
        # with cert.open("rb") as f:
        #     cert_contents = f.read()
        # with key.open("rb") as f:
        #     key_contents = f.read()
        self.ssl_ctx = (True, str(cert.resolve()), str(key.resolve()), password)

    def _init_shared_socket(self):
        self._shd = SocketHolder.from_address(self.bind_addr, self.bind_port, self.backlog)
        self._sfd = self._shd.get_fd()

    def signal_handler_interrupt(self, *args, **kwargs):
        self.interrupt_signal = True
        self.main_loop_interrupt.set()

    def signal_handler_reload(self, *args, **kwargs):
        self.reload_signal = True
        self.main_loop_interrupt.set()

    def _spawn_worker(self, idx, target, callback_loader, socket_loader) -> WT:
        raise NotImplementedError

    def _spawn_workers(self, spawn_target, target_loader):
        for idx in range(self.workers):
            wrk = self._spawn_worker(idx=idx, target=spawn_target, callback_loader=target_loader)
            wrk.start()
            self.wrks.append(wrk)

    def _respawn_workers(self, workers, spawn_target, target_loader, delay: float = 0):
        for idx in workers:
            self.respawned_wrks[idx] = time.time()
            logger.info(f'Respawning worker-{idx + 1}')
            old_wrk = self.wrks.pop(idx)
            wrk = self._spawn_worker(idx=idx, target=spawn_target, callback_loader=target_loader)
            wrk.start()
            self.wrks.insert(idx, wrk)
            time.sleep(delay)
            logger.info(f'Stopping old worker-{idx + 1}')
            old_wrk.terminate()
            old_wrk.join(self.workers_kill_timeout)
            if self.workers_kill_timeout:
                # the worker might still be reported alive after `join`, let's context switch
                if old_wrk.is_alive():
                    time.sleep(0.001)
                if old_wrk.is_alive():
                    logger.warning(f'Killing old worker-{idx + 1} after it refused to gracefully stop')
                    old_wrk.kill()
                    old_wrk.join()

    def _stop_workers(self):
        for wrk in self.wrks:
            wrk.terminate()

        for wrk in self.wrks:
            wrk.join(self.workers_kill_timeout)
            if self.workers_kill_timeout:
                # the worker might still be reported after `join`, let's context switch
                if wrk.is_alive():
                    time.sleep(0.001)
                if wrk.is_alive():
                    logger.warning(f'Killing worker-{wrk.idx} after it refused to gracefully stop')
                    wrk.kill()
                    wrk.join()

        self.wrks.clear()

    def _workers_lifetime_watcher(self, ttl):
        time.sleep(ttl)
        self.lifetime_signal = True
        self.main_loop_interrupt.set()

    def _watch_workers_lifetime(self, ttl):
        waker = threading.Thread(target=self._workers_lifetime_watcher, args=(ttl,), daemon=True)
        waker.start()

    def _write_pid(self):
        with self.pid_file.open('w') as pid_file:
            pid_file.write(str(self.pid))

    def _write_pidfile(self):
        if not self.pid_file:
            return

        existing_pid = None

        if self.pid_file.exists():
            try:
                with self.pid_file.open('r') as pid_file:
                    existing_pid = int(pid_file.read())
            except Exception:
                logger.error(f'Unable to read existing PID file {self.pid_file}')
                raise PidFileError

        if existing_pid is not None and existing_pid != self.pid:
            existing_process = True
            try:
                os.kill(existing_pid, 0)
            except OSError as e:
                if e.args[0] == errno.ESRCH:
                    existing_process = False

            if existing_process:
                logger.error(f'The PID file {self.pid_file} already exists for {existing_pid}')
                raise PidFileError

        self._write_pid()

    def _unlink_pidfile(self):
        if not (self.pid_file and self.pid_file.exists()):
            return

        try:
            with self.pid_file.open('r') as pid_file:
                file_pid = int(pid_file.read())
        except Exception:
            logger.error(f'Unable to read PID file {self.pid_file}')
            return

        if file_pid == self.pid:
            self.pid_file.unlink()

    def startup(self, spawn_target, target_loader):
        self.pid = os.getpid()
        logger.info(f'Starting granian (main PID: {self.pid})')
        self._write_pidfile()
        set_main_signals(self.signal_handler_interrupt, self.signal_handler_reload)
        self._init_shared_socket()
        proto = 'https' if self.ssl_ctx[0] else 'http'
        logger.info(f'Listening at: {proto}://{self.bind_addr}:{self.bind_port}')

        self._spawn_workers(spawn_target, target_loader)

        if self.workers_lifetime is not None:
            self._watch_workers_lifetime(self.workers_lifetime)

    def shutdown(self, exit_code=0):
        logger.info('Shutting down granian')
        self._stop_workers()
        self._unlink_pidfile()
        if not exit_code and self.interrupt_children:
            exit_code = 1
        if exit_code:
            sys.exit(exit_code)

    def _reload(self, spawn_target, target_loader):
        logger.info('HUP signal received, gracefully respawning workers..')
        workers = list(range(self.workers))
        self.reload_signal = False
        self.respawned_wrks.clear()
        self.main_loop_interrupt.clear()
        self._respawn_workers(workers, spawn_target, target_loader, delay=self.respawn_interval)

    def _serve_loop(self, spawn_target, target_loader):
        while True:
            self.main_loop_interrupt.wait()
            if self.interrupt_signal:
                break

            if self.interrupt_children:
                if not self.respawn_failed_workers:
                    break

                cycle = time.time()
                if any(cycle - self.respawned_wrks.get(idx, 0) <= 5.5 for idx in self.interrupt_children):
                    logger.error('Worker crash loop detected, exiting')
                    break

                workers = list(self.interrupt_children)
                self.interrupt_children.clear()
                self.respawned_wrks.clear()
                self.main_loop_interrupt.clear()
                self._respawn_workers(workers, spawn_target, target_loader)

            if self.reload_signal:
                self._reload(spawn_target, target_loader)

            if self.lifetime_signal:
                self.lifetime_signal = False
                self.main_loop_interrupt.clear()
                ttl = self.workers_lifetime * 0.95
                now = time.time()
                etas = [self.workers_lifetime]
                for worker in list(self.wrks):
                    if (now - worker.birth) >= ttl:
                        logger.info(f'worker-{worker.idx + 1} lifetime expired, gracefully respawning..')
                        self._respawn_workers([worker.idx], spawn_target, target_loader, delay=self.respawn_interval)
                    else:
                        elapsed = now - worker.birth
                        remaining = self.workers_lifetime - elapsed
                        etas.append(max(60, int(remaining)))
                next_tick = min(etas)
                self._watch_workers_lifetime(next_tick)

    def _serve(self, spawn_target, target_loader):
        self.startup(spawn_target, target_loader)
        self._serve_loop(spawn_target, target_loader)
        self.shutdown()

    def _serve_with_reloader(self, spawn_target, target_loader):
        if watchfiles is None:
            logger.error('Using --reload requires the granian[reload] extra')
            sys.exit(1)

        # Use given or default filter rules
        reload_filter = self.reload_filter or watchfiles.filters.DefaultFilter
        # Extend `reload_filter` with explicit args
        ignore_dirs = (*reload_filter.ignore_dirs, *self.reload_ignore_dirs)
        ignore_entity_patterns = (
            *reload_filter.ignore_entity_patterns,
            *self.reload_ignore_patterns,
        )
        ignore_paths = (*reload_filter.ignore_paths, *self.reload_ignore_paths)
        # Construct new filter
        reload_filter = watchfiles.filters.DefaultFilter(
            ignore_dirs=ignore_dirs, ignore_entity_patterns=ignore_entity_patterns, ignore_paths=ignore_paths
        )

        self.startup(spawn_target, target_loader)

        serve_loop = True
        while serve_loop:
            try:
                for changes in watchfiles.watch(
                    *self.reload_paths, watch_filter=reload_filter, stop_event=self.main_loop_interrupt
                ):
                    logger.info('Changes detected, reloading workers..')
                    for change, file in changes:
                        logger.info(f'{change.raw_str().capitalize()}: {file}')
                    self._stop_workers()
                    self._spawn_workers(spawn_target, target_loader)
            except StopIteration:
                pass

            if self.reload_signal:
                self._reload(spawn_target, target_loader)
            else:
                serve_loop = False

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
            target_loader = partial(load_target, self.target, factory=self.factory)

        if not spawn_target:
            spawn_target = default_spawners[self.interface]
            if sys.platform == 'win32' and self.workers > 1:
                self.workers = 1
                logger.warn(
                    'Due to a bug in Windows unblocking socket implementation '
                    "granian can't support multiple workers on this platform. "
                    'Number of workers will now fallback to 1.'
                )

        if self.interface != Interfaces.WSGI and self.blocking_threads > 1:
            logger.error('Blocking threads > 1 is not supported on ASGI and RSGI')
            raise ConfigurationError('blocking_threads')

        if self.websockets:
            if self.interface == Interfaces.WSGI:
                logger.info('Websockets are not supported on WSGI, ignoring')
            if self.http == HTTPModes.http2:
                logger.info('Websockets are not supported on HTTP/2 only, ignoring')

        if setproctitle is not None:
            self.process_name = self.process_name or (
                f'granian {self.interface} {self.bind_addr}:{self.bind_port} {self.target}'
            )
            setproctitle.setproctitle(self.process_name)
        elif self.process_name is not None:
            logger.error('Setting process name requires the granian[pname] extra')
            raise ConfigurationError('process_name')

        if self.workers_lifetime is not None:
            if self.reload_on_changes:
                logger.info('Workers lifetime is not available in combination with changes reloader, ignoring')
            if self.workers_lifetime < 60:
                logger.error('Workers lifetime cannot be less than 60 seconds')
                raise ConfigurationError('workers_lifetime')

        if self.blocking_threads_idle_timeout < 10 or self.blocking_threads_idle_timeout > 600:
            logger.error('Blocking threads idle timeout must be between 10 and 600 seconds')
            raise ConfigurationError('blocking_threads_idle_timeout')

        cpus = multiprocessing.cpu_count()
        if self.workers > cpus:
            logger.warning(
                'Configured number of workers appears to be higher than the amount of CPU cores available. '
                'Mind that such value might actually decrease the overall throughput of the server. '
                f'Consider using {cpus} workers and tune threads configuration instead'
            )
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

        serve_method = self._serve_with_reloader if self.reload_on_changes else self._serve
        serve_method(spawn_target, target_loader)
