import sys
import threading
from typing import Any

from ._types import WebsocketMessage
from .http import HTTP1Settings, HTTP2Settings

__version__: str
BUILD_GIL: bool

class RSGIHeaders:
    def __contains__(self, key: str) -> bool: ...
    def keys(self) -> list[str]: ...
    def values(self) -> list[str]: ...
    def items(self) -> list[tuple[str, str]]: ...
    def get(self, key: str, default: Any = None) -> Any: ...

class RSGIHTTPStreamTransport:
    async def send_bytes(self, data: bytes) -> None: ...
    async def send_str(self, data: str) -> None: ...

class RSGIHTTPProtocol:
    async def __call__(self) -> bytes: ...
    def __aiter__(self) -> Any: ...
    async def client_disconnect(self) -> None: ...
    def response_empty(self, status: int, headers: list[tuple[str, str]]) -> None: ...
    def response_str(self, status: int, headers: list[tuple[str, str]], body: str) -> None: ...
    def response_bytes(self, status: int, headers: list[tuple[str, str]], body: bytes) -> None: ...
    def response_file(self, status: int, headers: list[tuple[str, str]], file: str) -> None: ...
    def response_file_range(
        self, status: int, headers: list[tuple[str, str]], file: str, start: int, end: int
    ) -> None: ...
    def response_stream(self, status: int, headers: list[tuple[str, str]]) -> RSGIHTTPStreamTransport: ...

class RSGIWebsocketTransport:
    async def receive(self) -> WebsocketMessage: ...
    async def send_bytes(self, data: bytes) -> None: ...
    async def send_str(self, data: str) -> None: ...

class RSGIWebsocketProtocol:
    async def accept(self) -> RSGIWebsocketTransport: ...
    def close(self, status: int | None) -> tuple[int, bool]: ...

class RSGIProtocolError(RuntimeError): ...
class RSGIProtocolClosed(RuntimeError): ...

class WSGIScope:
    def to_environ(self, environ: dict[str, Any]) -> dict[str, Any]: ...

class WorkerSignal:
    def __init__(self) -> None: ...
    def set(self) -> None: ...

class WorkerSignalSync:
    qs: threading.Event

    def __init__(self) -> None: ...
    def set(self) -> None: ...

class ASGIWorker:
    def __new__(
        cls,
        worker_id: int,
        sock: Any,
        threads: int,
        blocking_threads: int,
        py_threads: int,
        py_threads_idle_timeout: int,
        backpressure: int,
        http_mode: str,
        http1_opts: HTTP1Settings | None,
        http2_opts: HTTP2Settings | None,
        websockets_enabled: bool,
        static_files: tuple[str, str, str | None, str | None] | None,
        ssl_enabled: bool,
        ssl_cert: str | None,
        ssl_key: str | None,
        ssl_key_password: str | None,
        ssl_protocol_min: str,
        ssl_ca: str | None,
        ssl_crl: list[str],
        ssl_client_verify: bool,
    ) -> ASGIWorker: ...

class WSGIWorker:
    def __new__(
        cls,
        worker_id: int,
        sock: Any,
        threads: int,
        blocking_threads: int,
        py_threads: int,
        py_threads_idle_timeout: int,
        backpressure: int,
        http_mode: str,
        http1_opts: HTTP1Settings | None,
        http2_opts: HTTP2Settings | None,
        static_files: tuple[str, str, str | None, str | None] | None,
        ssl_enabled: bool,
        ssl_cert: str | None,
        ssl_key: str | None,
        ssl_key_password: str | None,
        ssl_protocol_min: str,
        ssl_ca: str | None,
        ssl_crl: list[str],
        ssl_client_verify: bool,
    ) -> WSGIWorker: ...

class RSGIWorker:
    def __new__(
        cls,
        worker_id: int,
        sock: Any,
        threads: int,
        blocking_threads: int,
        py_threads: int,
        py_threads_idle_timeout: int,
        backpressure: int,
        http_mode: str,
        http1_opts: HTTP1Settings | None,
        http2_opts: HTTP2Settings | None,
        websockets_enabled: bool,
        static_files: tuple[str, str, str | None, str | None] | None,
        ssl_enabled: bool,
        ssl_cert: str | None,
        ssl_key: str | None,
        ssl_key_password: str | None,
        ssl_protocol_min: str,
        ssl_ca: str | None,
        ssl_crl: list[str],
        ssl_client_verify: bool,
    ) -> RSGIWorker: ...

class SocketHolder:
    def get_fd(self) -> Any: ...
    def is_uds(self) -> bool: ...

class ListenerSpec:
    def __new__(cls, host: str, port: int, backlog: int) -> ListenerSpec: ...
    def build(self) -> SocketHolder: ...

if sys.platform != 'win32':
    class UnixListenerSpec:
        def __new__(cls, bind: str, backlog: int, permissions: int | None) -> UnixListenerSpec: ...
        def build(self) -> SocketHolder: ...
        def is_uds(self) -> bool: ...

class CallbackScheduler:
    _loop: Any
    _ctx: Any

    def _run(self, coro: Any) -> None: ...

class ProcInfoCollector:
    def __init__(self) -> None: ...
    def memory(self, pids: list[int] | None = None) -> int: ...

class IPCReceiverHandle:
    def __init__(self, id: int, fd: int): ...
    def run(self): ...

class IPCSenderHandle:
    def __init__(self, fd: int): ...

class MetricsAggregator:
    def __init__(self, size: int): ...
    def incr_spawn(self, val: int): ...
    def incr_respawn_err(self, val: int): ...
    def incr_respawn_ttl(self, val: int): ...
    def incr_respawn_rss(self, val: int): ...

class MetricsExporter:
    def __init__(self, aggregator: MetricsAggregator): ...
    def run(self, sock: SocketHolder, sig: WorkerSignal): ...
