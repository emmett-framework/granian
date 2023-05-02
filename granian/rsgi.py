from enum import Enum
from typing import Union

from ._granian import (
    RSGIHTTPProtocol as HTTPProtocol,
    RSGIWebsocketProtocol as WebsocketProtocol,
    RSGIHeaders as Headers,
    RSGIScope as Scope,
    RSGIProtocolError as ProtocolError,
    RSGIProtocolClosed as ProtocolClosed
)


class WebsocketMessageType(int, Enum):
    close = 0
    bytes = 1
    string = 2


class WebsocketMessage:
    kind: WebsocketMessageType
    data: Union[bytes, str]
