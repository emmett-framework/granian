from enum import Enum

from .._granian import (
    RSGIHeaders as Headers,
    RSGIHTTPProtocol as HTTPProtocol,  # noqa: F401
    RSGIProtocolClosed as RSGIProtocolClosed,
    RSGIProtocolError as RSGIProtocolError,
    RSGIWebsocketProtocol as WebsocketProtocol,  # noqa: F401
)


class Scope:
    proto: str
    http_version: str
    rsgi_version: str
    server: str
    client: str
    scheme: str
    method: str
    path: str
    query_string: str
    authority: str | None

    @property
    def headers(self) -> Headers: ...


class WebsocketMessageType(int, Enum):
    close = 0
    bytes = 1
    string = 2


class WebsocketMessage:
    kind: WebsocketMessageType
    data: bytes | str
