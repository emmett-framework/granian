from typing import Any, Dict, List, Optional, Tuple, Protocol

from ._types import WebsocketMessage
from .http import HTTP1Settings, HTTP2Settings

__version__: str

class ASGIScope:
    def as_dict(self, root_path: str) -> Dict[str, Any]: ...

class RSGIHeaders:
    def __contains__(self, key: str) -> bool: ...
    def keys(self) -> List[str]: ...
    def values(self) -> List[str]: ...
    def items(self) -> List[Tuple[str]]: ...
    def get(self, key: str, default: Any = None) -> Any: ...

class RSGIScope:
    proto: str
    http_version: str
    rsgi_version: str
    server: str
    client: str
    scheme: str
    method: str
    path: str
    query_string: str
    authority: Optional[str]

    @property
    def headers(self) -> RSGIHeaders: ...

class RSGIHTTPStreamTransport:
    async def send_bytes(self, data: bytes): ...
    async def send_str(self, data: str): ...

class RSGIHTTPProtocol:
    async def __call__(self) -> bytes: ...
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

class __WorkerConfig(Protocol):
    def __init__(
        self,
        id: int,
        socket_fd: int,
        threads: int,
        pthreads: int,
        http_mode: str,
        http1_opts: HTTP1Settings,
        http2_opts: HTTP2Settings,
        websockets_enabled: bool,
        opt_enabled: bool,
        ssl_enabled: bool,
        ssl_cert: Optional[str],
        ssl_key: Optional[str],
    ):
        ...

class ASGIWorker(__WorkerConfig):
    ...

class RSGIWorker(__WorkerConfig):
    ...

class WSGIWorker(__WorkerConfig):
    ...


class ListenerHolder:
    @classmethod
    def from_address(cls, bind_addr: str, port: int, backlog: int) -> "ListenerHolder": ...
