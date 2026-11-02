import logging
import threading

from granian.server.common import AbstractWorker


class _Process:
    def __init__(self, exitcode):
        self.exitcode = exitcode

    def join(self, timeout=None):
        return None

    def is_alive(self):
        return False


class _Parent:
    def __init__(self):
        self.reload_on_changes = False
        self.reload_ignore_worker_failure = False
        self.interrupt_children = []
        self.crashed_children = []
        self.main_loop_interrupt = threading.Event()


class _Worker(AbstractWorker):
    def __init__(self, parent, exitcode):
        self.parent = parent
        self.idx = 0
        self.interrupt_by_parent = False
        self.inner = _Process(exitcode)

    def _spawn(self, target, args):
        raise NotImplementedError

    def _id(self):
        raise NotImplementedError


def _watch(exitcode, parent_stopped=False):
    parent = _Parent()
    worker = _Worker(parent, exitcode)
    worker.interrupt_by_parent = parent_stopped
    with _capture() as records:
        worker._watcher()
    return parent, records


class _ListHandler(logging.Handler):
    def __init__(self, records):
        super().__init__()
        self.records = records

    def emit(self, record):
        self.records.append(record)


class _capture:
    def __enter__(self):
        self.records = []
        self.logger = logging.getLogger('_granian')
        self.level = self.logger.level
        self.logger.setLevel(logging.DEBUG)
        self.handler = _ListHandler(self.records)
        self.logger.addHandler(self.handler)
        return self.records

    def __exit__(self, *args):
        self.logger.removeHandler(self.handler)
        self.logger.setLevel(self.level)


def test_clean_process_exit_is_not_a_crash():
    parent, records = _watch(0)

    assert parent.interrupt_children == [0]
    assert parent.crashed_children == []
    assert parent.main_loop_interrupt.is_set()
    assert any(record.levelno == logging.INFO and 'worker-1 exited' in record.getMessage() for record in records)
    assert not any('Unexpected exit' in record.getMessage() for record in records)


def test_failed_process_exit_is_a_crash():
    for exitcode in (1, -15, None):
        parent, records = _watch(exitcode)

        assert parent.interrupt_children == [0]
        assert parent.crashed_children == [0]
        assert any('Unexpected exit from worker-1' in record.getMessage() for record in records)


def test_parent_stop_is_not_a_worker_exit():
    parent, records = _watch(0, parent_stopped=True)

    assert parent.interrupt_children == []
    assert parent.crashed_children == []
    assert not parent.main_loop_interrupt.is_set()
    assert records == []
