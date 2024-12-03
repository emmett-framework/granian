import contextvars
from asyncio.tasks import _enter_task as _aio_taskenter, _leave_task as _aio_taskleave
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


class _CBScheduler(_BaseCBScheduler):
    __slots__ = []

    def __init__(self, loop, ctx, cb, aio_tenter, aio_texit):
        super().__init__()
        self._schedule_fn = _cbsched_schedule(loop, ctx, self._run, cb)


def _new_cbscheduler(loop, cb):
    return _CBScheduler(
        loop, contextvars.copy_context(), cb, partial(_aio_taskenter, loop), partial(_aio_taskleave, loop)
    )


def _cbsched_schedule(loop, ctx, run, cb):
    def _schedule(watcher):
        loop.call_soon_threadsafe(run, cb(watcher), context=ctx)

    return _schedule
