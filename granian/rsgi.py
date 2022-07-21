from collections import namedtuple
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


class ResponseType(int, Enum):
    empty = 0
    bytes = 1
    string = 2
    file_path = 10
    # chunks = 20


class WebsocketMessageType(int, Enum):
    close = 0
    bytes = 1
    string = 2


class WebsocketMessage:
    kind: WebsocketMessageType
    data: Union[bytes, str]


RSGIResponse = namedtuple(
    "RSGIResponse",
    ["mode", "status", "headers", "bytes_data", "str_data", "file_path"],
    defaults=[ResponseType.empty, 200, {}, None, None, None]
)


class Response:
    @classmethod
    def empty(cls, status=200, headers={}):
        return RSGIResponse(
            ResponseType.empty,
            status,
            headers
        )

    @classmethod
    def bytes(cls, data, status=200, headers={}):
        return RSGIResponse(
            ResponseType.bytes,
            status,
            headers,
            bytes_data=data
        )

    @classmethod
    def str(cls, data, status=200, headers={}):
        return RSGIResponse(
            ResponseType.string,
            status,
            headers,
            str_data=data
        )

    @classmethod
    def file(cls, data, status=200, headers={}):
        return RSGIResponse(
            ResponseType.file_path,
            status,
            headers,
            file_path=data
        )


def callback_wrapper(callback):
    @wraps(callback)
    def wrapper(
        watcher, scope: Scope, protocol: Union[HTTPProtocol, WebsocketProtocol]
    ):
        watcher.event_loop.call_soon_threadsafe(
            future_wrapper,
            watcher,
            callback(scope, protocol),
            future_handler,
            context=watcher.context
        )
    return wrapper


def future_handler(watcher):
    def handler(task):
        try:
            res = task.result()
        except Exception:
            res = None
        watcher.done(res)
    return handler
