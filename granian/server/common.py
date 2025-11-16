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
from .._imports import dotenv, setproctitle, watchfiles
from .._internal import build_env_loader, load_target
from .._signals import set_main_signals
from ..constants import HTTPModes, Interfaces, Loops, RuntimeModes, SSLProtocols, TaskImpl
from ..errors import ConfigurationError, PidFileError
from ..http import HTTP1Settings, HTTP2Settings
from ..log import DEFAULT_ACCESSLOG_FMT, LogLevels, configure_logging, logger
from ..net import SocketSpec, UnixSocketSpec


WT = TypeVar('WT')

WORKERS_METHODS = {
    RuntimeModes.mt: {False: 'serve_mtr', True: 'serve_mtr_uds'},
    RuntimeModes.st: {False: 'serve_str', True: 'serve_str_uds'},
}


class AbstractWorker:
    _idl = 'id'

    def __init__(self, parent: AbstractServer, idx: int, target: Any, args: Any):
        self.parent = parent
        self.idx = idx
        self.interrupt_by_parent = False
        self.birth = time.monotonic()
        self._spawn(target, args)

    def _spawn(self, target, args):
        raise NotImplementedError

    def _id(self):
        raise NotImplementedError

    def _watcher(self):
        self.inner.join()
        if not self.interrupt_by_parent:
            logger.error(f'Unexpected exit from worker-{self.idx + 1}')
            if self.parent.reload_on_changes and self.parent.reload_ignore_worker_failure:
                return
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
        uds: Optional[Path] = None,
        uds_permissions: Optional[int] = None,
        interface: Interfaces = Interfaces.RSGI,
        workers: int = 1,
        blocking_threads: Optional[int] = None,
        blocking_threads_idle_timeout: int = 30,
        runtime_threads: int = 1,
        runtime_blocking_threads: Optional[int] = None,
        runtime_mode: RuntimeModes = RuntimeModes.auto,
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
        ssl_protocol_min: SSLProtocols = SSLProtocols.tls13,
        ssl_ca: Optional[Path] = None,
        ssl_crl: Optional[List[Path]] = None,
        ssl_client_verify: bool = False,
        url_path_prefix: Optional[str] = None,
        respawn_failed_workers: bool = False,
        respawn_interval: float = 3.5,
        rss_sample_interval: int = 30,
        rss_samples: int = 1,
        workers_lifetime: Optional[int] = None,
        workers_max_rss: Optional[int] = None,
        workers_kill_timeout: Optional[int] = None,
        factory: bool = False,
        working_dir: Optional[Path] = None,
        env_files: Optional[Sequence[Path]] = None,
        static_path_route: str = '/static',
        static_path_mount: Optional[Path] = None,
        static_path_expires: int = 86400,
        reload: bool = False,
        reload_paths: Optional[Sequence[Path]] = None,
        reload_ignore_dirs: Optional[Sequence[str]] = None,
        reload_ignore_patterns: Optional[Sequence[str]] = None,
        reload_ignore_paths: Optional[Sequence[Path]] = None,
        reload_filter: Optional[Type[watchfiles.BaseFilter]] = None,
        reload_tick: int = 50,
        reload_ignore_worker_failure: bool = False,
        process_name: Optional[str] = None,
        pid_file: Optional[Path] = None,
    ):
        self.target = target
        self.bind_addr = address
        self.bind_port = port
        self.bind_uds = uds.resolve() if uds else None
        self.uds_permissions = uds_permissions
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
        self.rss_sample_interval = rss_sample_interval
        self.rss_samples = rss_samples
        self._rss_wrk_samples = {}
        self.workers_lifetime = workers_lifetime
        self.workers_rss = workers_max_rss * 1024 * 1024 if workers_max_rss else None
        self.workers_kill_timeout = workers_kill_timeout
        self.factory = factory
        self.working_dir = working_dir
        self.env_files = env_files or ()
        self.static_path = (
            (
                static_path_route,
                str(static_path_mount.resolve()),
                (str(static_path_expires) if static_path_expires else None),
            )
            if static_path_mount
            else None
        )
        self.reload_paths = reload_paths or [Path.cwd()]
        self.reload_ignore_paths = reload_ignore_paths or ()
        self.reload_ignore_dirs = reload_ignore_dirs or ()
        self.reload_ignore_patterns = reload_ignore_patterns or ()
        self.reload_filter = reload_filter
        self.reload_tick = reload_tick
        self.reload_ignore_worker_failure = reload_ignore_worker_failure
        self.process_name = process_name
        self.pid_file = pid_file

        self.hooks_startup = []
        self.hooks_reload = []
        self.hooks_shutdown = []

        configure_logging(self.log_level, self.log_config, self.log_enabled)

        self.build_ssl_context(
            ssl_cert, ssl_key, ssl_key_password, ssl_protocol_min, ssl_ca, ssl_crl or [], ssl_client_verify
        )
        self._ssp = None
        self._shd = None
        self._sfd = None
        self.wrks: List[WT] = []
        self.main_loop_interrupt = threading.Event()
        self.interrupt_signal = False
        self.interrupt_children = []
        self.respawned_wrks = {}
        self.reload_signal = False
        self.lifetime_signal = False
        self.rss_signal = False
        self.pid = None
        self._env_loader = build_env_loader()

    def build_ssl_context(
        self,
        cert: Optional[Path],
        key: Optional[Path],
        password: Optional[str],
        proto: SSLProtocols,
        ca: Optional[Path],
        crl: List[Path],
        client_verify: bool,
    ):
        if not (cert and key):
            self.ssl_ctx = (False, None, None, None, str(proto), None, [], False)
            return
        # uneeded?
        ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        ctx.load_cert_chain(str(cert.resolve()), str(key.resolve()), password)
        #: build ctx
        if client_verify and not ca:
            logger.warning('SSL client verification requires a CA certificate, ignoring')
            client_verify = False
        self.ssl_ctx = (
            True,
            str(cert.resolve()),
            str(key.resolve()),
            password,
            str(proto),
            str(ca.resolve()) if ca else None,
            [str(item.resolve()) for item in crl],
            client_verify,
        )

    @property
    def _bind_addr_fmt(self):
        return f'unix:{self.bind_uds}' if self.bind_uds else f'{self.bind_addr}:{self.bind_port}'

    @staticmethod
    def _call_hooks(hooks):
        for hook in hooks:
            hook()

    def on_startup(self, hook: Callable[[], Any]) -> Callable[[], Any]:
        self.hooks_startup.append(hook)
        return hook

    def on_reload(self, hook: Callable[[], Any]) -> Callable[[], Any]:
        self.hooks_reload.append(hook)
        return hook

    def on_shutdown(self, hook: Callable[[], Any]) -> Callable[[], Any]:
        self.hooks_shutdown.append(hook)
        return hook

    def _init_shared_socket(self):
        if self.bind_uds:
            self._ssp = UnixSocketSpec(str(self.bind_uds), self.backlog, self.uds_permissions)
        else:
            self._ssp = SocketSpec(self.bind_addr, self.bind_port, self.backlog)
        self._shd = self._ssp.build()
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
            self.respawned_wrks[idx] = time.monotonic()
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

    def _workers_rss_watcher(self):
        time.sleep(self.rss_sample_interval)
        self.rss_signal = True
        self.main_loop_interrupt.set()

    def _watch_workers_lifetime(self, ttl):
        waker = threading.Thread(target=self._workers_lifetime_watcher, args=(ttl,), daemon=True)
        waker.start()

    def _watch_workers_rss(self):
        waker = threading.Thread(target=self._workers_rss_watcher, daemon=True)
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
        if self.bind_uds and self.bind_uds.exists():
            self.bind_uds.unlink()

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
        logger.info(f'Listening at: {proto}://{self._bind_addr_fmt}')

        self._env_loader(self.env_files)
        self._call_hooks(self.hooks_startup)
        self._spawn_workers(spawn_target, target_loader)

        if self.workers_lifetime is not None:
            self._watch_workers_lifetime(self.workers_lifetime)
        if self.workers_rss is not None:
            self._watch_workers_rss()

    def shutdown(self, exit_code=0):
        logger.info('Shutting down granian')
        self._stop_workers()
        self._call_hooks(self.hooks_shutdown)
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

        self._env_loader(self.env_files)
        self._call_hooks(self.hooks_reload)
        return self._respawn_workers(workers, spawn_target, target_loader, delay=self.respawn_interval)

    def _handle_rss_signal(self, spawn_target, target_loader):
        raise NotImplementedError

    def _serve_loop(self, spawn_target, target_loader):
        while True:
            self.main_loop_interrupt.wait()
            if self.interrupt_signal:
                break

            if self.interrupt_children:
                if not self.respawn_failed_workers:
                    break

                cycle = time.monotonic()
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

            if self.lifetime_signal or self.rss_signal:
                self.main_loop_interrupt.clear()

                if self.lifetime_signal:
                    self.lifetime_signal = False
                    ttl = self.workers_lifetime * 0.95
                    now = time.monotonic()
                    etas = [self.workers_lifetime]
                    for worker in list(self.wrks):
                        if (now - worker.birth) >= ttl:
                            logger.info(f'worker-{worker.idx + 1} lifetime expired, gracefully respawning..')
                            self._respawn_workers(
                                [worker.idx], spawn_target, target_loader, delay=self.respawn_interval
                            )
                        else:
                            elapsed = now - worker.birth
                            remaining = self.workers_lifetime - elapsed
                            etas.append(max(60, int(remaining)))
                    next_tick = min(etas)
                    self._watch_workers_lifetime(next_tick)

                if self.rss_signal:
                    self.rss_signal = False
                    self._handle_rss_signal(spawn_target, target_loader)
                    self._watch_workers_rss()

    def _serve(self, spawn_target, target_loader):
        self.startup(spawn_target, target_loader)
        self._serve_loop(spawn_target, target_loader)
        self.shutdown()

    def _serve_with_reloader(self, spawn_target, target_loader):
        if watchfiles is None:
            logger.error('Using --reload requires the granian[reload] extra')
            sys.exit(1)

        # Use given or default filter rules
        reload_filter_cls = self.reload_filter or watchfiles.filters.DefaultFilter
        # Extend `reload_filter` with provided args
        reload_filter_cls.ignore_dirs = (*reload_filter_cls.ignore_dirs, *self.reload_ignore_dirs)
        reload_filter_cls.ignore_entity_patterns = (
            *reload_filter_cls.ignore_entity_patterns,
            *self.reload_ignore_patterns,
        )
        reload_filter_cls.ignore_paths = (*reload_filter_cls.ignore_paths, *self.reload_ignore_paths)
        # Construct new filter
        reload_filter = reload_filter_cls()

        self.startup(spawn_target, target_loader)

        serve_loop = True
        while serve_loop:
            try:
                for changes in watchfiles.watch(
                    *self.reload_paths,
                    watch_filter=reload_filter,
                    stop_event=self.main_loop_interrupt,
                    step=self.reload_tick,
                ):
                    logger.info('Changes detected, reloading workers..')
                    for change, file in changes:
                        logger.info(f'{change.raw_str().capitalize()}: {file}')
                    self._env_loader(self.env_files)
                    self._call_hooks(self.hooks_reload)
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
            target_loader = partial(load_target, self.target, wd=self.working_dir, factory=self.factory)

        if not spawn_target:
            spawn_target = default_spawners[self.interface]
            if sys.platform == 'win32' and self.workers > 1:
                self.workers = 1
                logger.warn(
                    'Due to a bug in Windows unblocking socket implementation '
                    "granian can't support multiple workers on this platform. "
                    'Number of workers will now fallback to 1.'
                )

        if self.bind_uds and sys.platform == 'win32':
            logger.error('Unix Domain sockets are not available on Windows')
            raise ConfigurationError('uds')

        if self.interface != Interfaces.WSGI and self.blocking_threads > 1:
            logger.error('Blocking threads > 1 is not supported on ASGI and RSGI')
            raise ConfigurationError('blocking_threads')

        if self.websockets:
            if self.interface == Interfaces.WSGI:
                self.websockets = False
                logger.info('Websockets are not supported on WSGI, ignoring')
            if self.http == HTTPModes.http2:
                logger.info('Websockets are not supported on HTTP/2 only, ignoring')

        if setproctitle is not None:
            self.process_name = self.process_name or (f'granian {self.interface} {self._bind_addr_fmt} {self.target}')
            setproctitle.setproctitle(self.process_name)
        elif self.process_name is not None:
            logger.error('Setting process name requires the granian[pname] extra')
            raise ConfigurationError('process_name')

        if self.env_files and dotenv is None:
            logger.error('Environment file(s) usage requires the granian[dotenv] extra')
            raise ConfigurationError('env_files')

        if self.workers_lifetime is not None:
            if self.reload_on_changes:
                self.workers_lifetime = None
                logger.info('Workers lifetime is not available in combination with changes reloader, ignoring')
            if self.workers_lifetime < 60:
                logger.error('Workers lifetime cannot be less than 60 seconds')
                raise ConfigurationError('workers_lifetime')

        if self.workers_rss is not None:
            if self.reload_on_changes:
                self.workers_rss = None
                logger.info('The resource monitor is not available in combination with changes reloader, ignoring')

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

        if self.runtime_mode == RuntimeModes.auto:
            self.runtime_mode = RuntimeModes.st
            if self.interface == Interfaces.WSGI:
                self.runtime_mode = RuntimeModes.mt
            if self.http == HTTPModes.http2:
                self.runtime_mode = RuntimeModes.mt

        if self.task_impl == TaskImpl.rust:
            if _PYV >= _PY_312:
                self.task_impl = TaskImpl.asyncio
                logger.warning('Rust task implementation is not available on Python >= 3.12, falling back to asyncio')
            else:
                logger.warning('Rust task implementation is experimental!')

        serve_method = self._serve_with_reloader if self.reload_on_changes else self._serve
        serve_method(spawn_target, target_loader)
