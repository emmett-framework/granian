import signal
import sys
import threading

from ._granian import WorkerSignal, WorkerSignalSync


def _get_signals():
    rv = [signal.SIGINT, signal.SIGTERM]
    if sys.platform == 'win32':
        rv.append(signal.SIGBREAK)
    return rv


def set_main_signals(interrupt_handler, reload_handler=None):
    for sig in _get_signals():
        signal.signal(sig, interrupt_handler)

    if reload_handler is not None and sys.platform != 'win32':
        signal.signal(signal.SIGHUP, reload_handler)


def set_loop_signals(loop):
    signal_event = WorkerSignal()

    def signal_handler(signum, frame):
        signal_event.set()

    signals = _get_signals()
    try:
        for sigval in signals:
            loop.add_signal_handler(sigval, signal_handler, sigval, None)
    except NotImplementedError:
        for sigval in signals:
            signal.signal(sigval, signal_handler)

    return signal_event


def set_sync_signals():
    signal_event = WorkerSignalSync(threading.Event())

    def signal_handler(signum, frame):
        signal_event.set()

    set_main_signals(signal_handler)
    return signal_event
