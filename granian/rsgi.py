from enum import Enum
from typing import Union

from ._granian import (
    RSGIHeaders as Headers,  # noqa
    RSGIHTTPProtocol as HTTPProtocol,  # noqa
    RSGIProtocolClosed as ProtocolClosed,  # noqa
    RSGIProtocolError as ProtocolError,  # noqa
    RSGIScope as Scope,  # noqa
    RSGIWebsocketProtocol as WebsocketProtocol,  # noqa
)


class WebsocketMessageType(int, Enum):
    close = 0
    bytes = 1
    string = 2


class WebsocketMessage:
    kind: WebsocketMessageType
    data: Union[bytes, str]
