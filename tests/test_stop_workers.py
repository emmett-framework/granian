import time
from types import SimpleNamespace
from typing import cast

from granian.server.common import AbstractServer


class _Worker:
    def __init__(self, idx, exit_at, clock):
        self.idx = idx
        self.exit_at = exit_at
        self.clock = clock
        self.killed = False
        self.join_timeout = None

    def terminate(self):
        return None

    def is_alive(self):
        return self.clock[0] < self.exit_at

    def join(self, timeout=None):
        self.join_timeout = timeout
        if timeout is None:
            self.clock[0] = self.exit_at
            return
        self.clock[0] = min(self.clock[0] + timeout, self.exit_at)

    def kill(self):
        self.killed = True


def test_stop_workers_does_not_spend_elapsed_time_twice(monkeypatch):
    clock = [1000.0]
    monkeypatch.setattr(time, 'monotonic', lambda: clock[0])
    monkeypatch.setattr(time, 'sleep', lambda _seconds: None)
    workers = [_Worker(idx, exit_at, clock) for idx, exit_at in enumerate((1001.0, 1004.0, 1006.0, 1009.0))]
    held = list(workers)
    server = SimpleNamespace(workers_kill_timeout=10, wrks=workers)

    AbstractServer._stop_workers(cast(AbstractServer, server))

    assert [worker.killed for worker in held] == [False, False, False, False]
    assert held[-1].join_timeout is not None
    assert held[-1].join_timeout > 3
