from typing import Any, Dict, List, Optional, Tuple

from ._types import WebsocketMessage

__version__: str

class RSGIHeaders:
    def __contains__(self, key: str) -> bool: ...
    def keys(self) -> List[str]: ...
    def values(self) -> List[str]: ...
    def items(self) -> List[Tuple[str]]: ...
    def get(self, key: str, default: Any = None) -> Any: ...

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
