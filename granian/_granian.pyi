from typing import Any, Dict, List, Tuple, Optional

from ._types import WebsocketMessage


class ASGIScope:
    client_ip: str
    client_port: int
    server_ip: str
    server_port: int
    headers: Dict[bytes, bytes]
    http_version: str
    method: str
    path: str
    proto: str
    query_string: str
    scheme: str


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

    @property
    def headers(self) -> RSGIHeaders: ...


class RSGIHTTPProtocol:
    async def __call__(self) -> bytes: ...
    def response_empty(self, status: int, headers: List[Tuple[str, str]]): ...
    def response_str(self, status: int, headers: List[Tuple[str, str]], body: str): ...
    def response_bytes(self, status: int, headers: List[Tuple[str, str]], body: bytes): ...
    def response_file(self, status: int, headers: List[Tuple[str, str]], file: str): ...


class RSGIWebsocketTransport:
    async def receive(self) -> WebsocketMessage: ...
    async def send_bytes(self, data: bytes): ...
    async def send_str(self, data: str): ...


class RSGIWebsocketProtocol:
    async def accept(self) -> RSGIWebsocketTransport: ...
    def close(self, status: Optional[int]) -> Tuple[int, bool]: ...
