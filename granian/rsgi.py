from collections import namedtuple
from enum import Enum
from functools import wraps

from . import _rsgi
from ._futures import future_wrapper

Headers = _rsgi.Headers
Scope = _rsgi.Scope


class ResponseType(int, Enum):
    empty = 0
    bytes = 1
    string = 2
    file_path = 10
    # chunks = 20


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
            # str_data=None,
            # file_path=None
        )

    @classmethod
    def str(cls, data, status=200, headers={}):
        return RSGIResponse(
            ResponseType.string,
            status,
            headers,
            # bytes_data=None,
            str_data=data,
            # file_path=None
        )

    @classmethod
    def file(cls, data, status=200, headers={}):
        return RSGIResponse(
            ResponseType.file_path,
            status,
            headers,
            # bytes_data=None,
            # str_data=None,
            file_path=data
        )


def callback_wrapper(callback):
    @wraps(callback)
    def wrapper(watcher, scope, transport):
        watcher.event_loop.call_soon_threadsafe(
            future_wrapper,
            watcher,
            callback(scope, transport),
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
