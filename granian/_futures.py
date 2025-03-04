from asyncio.tasks import (
    Task as _Task,
    _enter_task as _aio_taskenter,
    _leave_task as _aio_taskleave,
    _register_task as _aio_taskreg,
)
from functools import partial

from ._granian import CallbackScheduler as _BaseCBScheduler


def _future_watcher_wrapper(inner):
    async def future_watcher(watcher):
        try:
            await inner(watcher.scope, watcher.proto)
        except BaseException as exc:
            watcher.err(exc)
            return
        watcher.done()

    return future_watcher


class _CBSchedulerTask(_Task):
    def __init__(self, *args, **kwargs):
        _aio_taskreg(self)

    def __await__(self):
        yield self

    def done(self):
        return False

    def result(self):
        raise RuntimeError('Result is not ready.')

    def exception(self):
        return None

    def cancel(self, msg=None):
        return False

    def cancelling(self):
        return 0

    def uncancel(self):
        return 0


class _CBScheduler(_BaseCBScheduler):
    __slots__ = []

    def __init__(self, loop, cb, aio_task, aio_tenter, aio_texit):
        super().__init__()
        self._schedule_fn = _cbsched_schedule(loop, self._ctx, self._run, cb)

    def cancel(self):
        return False

    def cancelling(self):
        return 0

    def uncancel(self):
        return 0


class _CBSchedulerAIO(_BaseCBScheduler):
    __slots__ = []

    def __init__(self, loop, cb, aio_task, aio_tenter, aio_texit):
        super().__init__()
        self._schedule_fn = _cbsched_aioschedule(loop, self._ctx, cb)


def _new_cbscheduler(loop, cb, impl_asyncio=False):
    if impl_asyncio:
        _cls, _task = _CBSchedulerAIO, None
    else:
        _cls, _task = _CBScheduler, _CBSchedulerTask()
    return _cls(loop, cb, _task, partial(_aio_taskenter, loop), partial(_aio_taskleave, loop))


def _cbsched_schedule(loop, ctx, run, cb):
    def _schedule(watcher):
        loop.call_soon_threadsafe(run, cb(watcher), context=ctx)

    return _schedule


def _cbsched_aioschedule(loop, ctx, cb):
    def _run(coro, watcher):
        task = loop.create_task(coro)
        watcher.taskref(task)

    def _schedule(watcher):
        loop.call_soon_threadsafe(_run, cb(watcher), watcher, context=ctx)

    return _schedule
