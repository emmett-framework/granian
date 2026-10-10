import asyncio

from granian.server.common import RuntimeModes, TaskImpl
from granian.server.mt import MTServer


class _Sock:
    def is_uds(self):
        return False


class _Signal:
    def __init__(self):
        self.callbacks = []

    def add_cb(self, callback):
        self.callbacks.append(callback)


class _Worker:
    def serve_mtr(self, scheduler, loop, shutdown_event):
        for callback in list(shutdown_event.callbacks):
            callback()


def _call_asgi_worker(monkeypatch, worker_factory, loop):
    monkeypatch.setattr('granian.server.mt.loops', type('Loops', (), {'get': staticmethod(lambda _impl: loop)})())
    monkeypatch.setattr('granian.server.mt.ASGIWorker', worker_factory)
    monkeypatch.setattr('granian.server.mt._future_watcher_wrapper', lambda inner: inner)
    monkeypatch.setattr('granian.server.mt._asgi_call_wrap', lambda *args, **kwargs: None)
    monkeypatch.setattr('granian.server.mt._new_cbscheduler', lambda *args, **kwargs: None)
    MTServer._spawn_asgi_worker(
        1,
        _Signal(),
        lambda scope, receive, send: None,
        (_Sock(), None),
        'asyncio',
        RuntimeModes.mt,
        1,
        None,
        1,
        10,
        1,
        TaskImpl.asyncio,
        'auto',
        None,
        None,
        False,
        None,
        None,
        (None, None),
        {},
        None,
    )


def test_asgi_worker_closes_loop(monkeypatch):
    loop = asyncio.new_event_loop()
    _call_asgi_worker(monkeypatch, lambda *args, **kwargs: _Worker(), loop)
    assert loop.is_closed()


def test_asgi_worker_closes_loop_when_worker_fails(monkeypatch):
    loop = asyncio.new_event_loop()

    def fail(*args, **kwargs):
        raise RuntimeError('worker failed')

    try:
        _call_asgi_worker(monkeypatch, fail, loop)
    except RuntimeError as exc:
        assert str(exc) == 'worker failed'
    else:
        raise AssertionError('worker error did not surface')
    assert loop.is_closed()
