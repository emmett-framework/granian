from enum import Enum
from functools import wraps
from typing import Union

from ._futures import future_with_watcher
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


def _callback_wrapper(callback):
    @wraps(callback)
    def wrapper(watcher, scope: Scope):
        watcher.event_loop.call_soon_threadsafe(
            future_with_watcher,
            callback(scope, watcher.proto),
            watcher,
            context=watcher.context
        )
    return wrapper
