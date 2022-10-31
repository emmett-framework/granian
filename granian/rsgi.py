from enum import Enum
from functools import wraps
from typing import Union

from ._futures import future_wrapper
from ._granian import (
    RSGIHTTPProtocol as HTTPProtocol,
    RSGIWebsocketProtocol as WebsocketProtocol,
    RSGIHeaders as Headers,
    RSGIScope as Scope
)


class WebsocketMessageType(int, Enum):
    close = 0
    bytes = 1
    string = 2


class WebsocketMessage:
    kind: WebsocketMessageType
    data: Union[bytes, str]


def callback_wrapper(callback):
    @wraps(callback)
    def wrapper(watcher, scope: Scope):
        watcher.event_loop.call_soon_threadsafe(
            future_wrapper,
            callback(scope, watcher.proto),
            watcher,
            context=watcher.context
        )
    return wrapper
