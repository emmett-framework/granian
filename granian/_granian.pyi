import sys
import threading
from typing import Any, Dict, List, Optional, Tuple

from ._types import WebsocketMessage
from .http import HTTP1Settings, HTTP2Settings

__version__: str
BUILD_GIL: bool

class RSGIHeaders:
    def __contains__(self, key: str) -> bool: ...
    def keys(self) -> List[str]: ...
    def values(self) -> List[str]: ...
    def items(self) -> List[Tuple[str, str]]: ...
    def get(self, key: str, default: Any = None) -> Any: ...

class RSGIHTTPStreamTransport:
    async def send_bytes(self, data: bytes): ...
    async def send_str(self, data: str): ...

class RSGIHTTPProtocol:
    async def __call__(self) -> bytes: ...
    def __aiter__(self) -> Any: ...
    async def client_disconnect(self) -> None: ...
    def response_empty(self, status: int, headers: List[Tuple[str, str]]): ...
    def response_str(self, status: int, headers: List[Tuple[str, str]], body: str): ...
    def response_bytes(self, status: int, headers: List[Tuple[str, str]], body: bytes): ...
    def response_file(self, status: int, headers: List[Tuple[str, str]], file: str): ...
    def response_stream(self, status: int, headers: List[Tuple[str, str]]) -> RSGIHTTPStreamTransport: ...

class RSGIWebsocketTransport:
    async def receive(self) -> WebsocketMessage: ...
    async def send_bytes(self, data: bytes): ...
    async def send_str(self, data: str): ...

class RSGIWebsocketProtocol:
    async def accept(self) -> RSGIWebsocketTransport: ...
    def close(self, status: Optional[int]) -> Tuple[int, bool]: ...

class RSGIProtocolError(RuntimeError): ...
class RSGIProtocolClosed(RuntimeError): ...

class WSGIScope:
    def to_environ(self, environ: Dict[str, Any]) -> Dict[str, Any]: ...

class WorkerSignal:
    def __init__(self): ...
    def set(self): ...

class WorkerSignalSync:
    qs: threading.Event

    def __init__(self): ...
    def set(self): ...

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
        http1_opts: Optional[HTTP1Settings],
        http2_opts: Optional[HTTP2Settings],
        websockets_enabled: bool,
        static_files: Optional[Tuple[str, str, str]],
        ssl_enabled: bool,
        ssl_cert: Optional[str],
        ssl_key: Optional[str],
        ssl_ca: Optional[str],
        ssl_crl: List[str],
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
        http1_opts: Optional[HTTP1Settings],
        http2_opts: Optional[HTTP2Settings],
        static_files: Optional[Tuple[str, str, str]],
        ssl_enabled: bool,
        ssl_cert: Optional[str],
        ssl_key: Optional[str],
        ssl_ca: Optional[str],
        ssl_crl: List[str],
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
        http1_opts: Optional[HTTP1Settings],
        http2_opts: Optional[HTTP2Settings],
        websockets_enabled: bool,
        static_files: Optional[Tuple[str, str, str]],
        ssl_enabled: bool,
        ssl_cert: Optional[str],
        ssl_key: Optional[str],
        ssl_ca: Optional[str],
        ssl_crl: List[str],
        ssl_client_verify: bool,
    ) -> RSGIWorker: ...

class SocketHolder:
    def get_fd(self) -> Any: ...
    def is_uds(self) -> bool: ...

class ListenerSpec:
    def __new__(cls, host: str, port: int, backlog: int) -> ListenerSpec: ...
    def build(self) -> SocketHolder: ...

class CallbackScheduler:
    _loop: Any
    _ctx: Any

    def _run(self, coro: Any): ...

class ProcInfoCollector:
    def __init__(self): ...
    def memory(self, pids: Optional[List[int]] = None) -> int: ...

if sys.platform != 'win32':
    class UnixListenerSpec:
        def __new__(cls, bind: str, backlog: int) -> UnixListenerSpec: ...
        def build(self) -> SocketHolder: ...
        def is_uds(self) -> bool: ...
