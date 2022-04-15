from collections import namedtuple
from enum import Enum
from functools import wraps

from . import _rsgi
from ._futures import future_wrapper

Scope = _rsgi.Scope
Receiver = _rsgi.Receiver


class ResponseType(int, Enum):
    empty = 0
    bytes = 1
    string = 2
    file_path = 10
    chunks = 20


Response = namedtuple(
    "Response",
    ["mode", "status", "headers", "bytes_data", "str_data", "file_path"],
    defaults=[ResponseType.empty, 200, {}, None, None, None]
)


def callback_wrapper(callback):
    @wraps(callback)
    def wrapper(watcher, scope, receiver):
        watcher.event_loop.call_soon_threadsafe(
            future_wrapper,
            watcher,
            callback(scope, receiver),
            future_handler,
            context=watcher.context
        )
    return wrapper


def future_handler(watcher):
    def handler(task):
        try:
            res = task.result()
            watcher.done(res)
        except Exception:
            watcher.done(None)
    return handler
