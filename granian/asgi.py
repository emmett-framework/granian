from functools import wraps

from . import _asgi
from ._futures import future_wrapper
from .io import Receiver

Sender = _asgi.Sender
Scope = _asgi.Scope


def receiver_wrapper(receiver):
    @wraps(receiver)
    async def wrapper():
        return {
            "type": "http.request",
            "body": await receiver,
            "more_body": False
        }
    return wrapper


def sender_wrapper(sender):
    @wraps(sender)
    async def wrapper(data):
        sender(data)
    return wrapper


def callback_wrapper(callback):
    @wraps(callback)
    def wrapper(watcher, scope, receiver, sender):
        client_addr, client_port = (scope.client.split(":") + ["0"])[:2]
        coro = callback(
            {
                "type": scope.proto,
                "asgi": {
                    "version": "3.0",
                    "spec_version": "2.3"
                },
                "http_version": scope.http_version,
                "server": ("127.0.0.1", 8000),
                "client": (client_addr, int(client_port)),
                "scheme": scope.scheme,
                "method": scope.method,
                "root_path": "",
                "path": scope.path,
                "raw_path": scope.path.encode("ascii"),
                "query_string": scope.query_string,
                "headers": scope.headers,
                "extensions": {}
            },
            receiver_wrapper(receiver),
            sender_wrapper(sender)
        )
        watcher.event_loop.call_soon_threadsafe(
            future_wrapper,
            watcher,
            coro,
            future_handler,
            context=watcher.context
        )
    return wrapper


def future_handler(watcher):
    def handler(task):
        try:
            task.result()
            watcher.done(True)
        except Exception:
            watcher.done(False)
    return handler
