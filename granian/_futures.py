from asyncio import get_running_loop as _aio_running_loop
from asyncio.tasks import (
    Task as _Task,
    _enter_task as _aio_taskenter,
    _leave_task as _aio_taskleave,
    _register_task as _aio_taskreg,
)
from functools import partial
from types import coroutine as _coroutine

from ._granian import CallbackScheduler as _BaseCBScheduler, request_acct_var as _request_acct_var


#: Set (by the fair scheduler only) in the context of every tracked request.
_acct_var = _request_acct_var()


def _future_watcher_wrapper(inner):
    async def future_watcher(watcher):
        try:
            acct = _acct_var.get(None)
            if acct is None:
                await inner(watcher.scope, watcher.proto)
            else:
                await _fair_steps(inner(watcher.scope, watcher.proto), acct)
        except BaseException as exc:
            watcher.err(exc)
            return
        watcher.done()

    return future_watcher


def _lane_dispatch_soon(loop, acct):
    """Second hop of the post-release dispatch (see `_lane_release`)."""
    loop.call_soon(acct.lane_dispatch)


def _lane_release(loop, acct):
    """Give the loop lane back, scheduling the next dispatch if requests are waiting.

    The dispatch is two hops away: the step that just ended may itself have made tasks
    ready (including its own, on a bare `yield` such as `asyncio.sleep(0)`), and those
    callbacks are queued after this point, so they must run and queue their turns first.
    """
    if acct.lane_release():
        loop.call_soon(_lane_dispatch_soon, loop, acct)


@_coroutine
def _lane_turn(loop, acct):
    """Wait for the request's turn on the event loop; return `(holding_lane, error)`.

    The turn is never taken on the spot, even when the lane is idle: a single-threaded loop
    runs one step at a time, so "nobody is waiting right now" only means the other ready
    requests have not been resumed yet. The dispatch is scheduled with `call_soon` instead,
    and runs after every wakeup already in the ready queue, i.e. once all of them queued.
    """
    fut = loop.create_future()
    if acct.lane_wait(fut):
        loop.call_soon(acct.lane_dispatch)
    try:
        yield from fut
    except GeneratorExit:
        if fut.done() and not fut.cancelled():
            try:
                _lane_release(loop, acct)
            except RuntimeError:
                # the loop is closed: nobody is left to dispatch to
                pass
        else:
            fut.cancel()
        raise
    except BaseException as exc:
        # cancelled while waiting: the turn may still have been granted in the meantime
        return fut.done() and not fut.cancelled(), exc
    return True, None


@_coroutine
def _fair_steps(coro, acct):
    """Drive a tracked request one step at a time, each step holding the loop lane.

    The task stays a regular asyncio task: the futures the app awaits are passed through
    untouched. But before resuming the app, the request waits for its turn from the fair
    scheduler, and it hands the lane back as soon as the step suspends, so the scheduler
    (and its per-flow budgets) decides which request's step the loop runs next, instead
    of the loop's FIFO ready queue.
    """
    loop = _aio_running_loop()
    send, throw = coro.send, coro.throw
    holding, exc = yield from _lane_turn(loop, acct)
    try:
        while True:
            try:
                if exc is None:
                    awaited = send(None)
                else:
                    err, exc = exc, None
                    awaited = throw(err)
            except StopIteration as stop:
                return stop.value
            if holding:
                holding = False
                _lane_release(loop, acct)
            try:
                yield awaited
            except GeneratorExit:
                coro.close()
                raise
            except BaseException as err:
                exc = err
            holding, turn_exc = yield from _lane_turn(loop, acct)
            if turn_exc is not None:
                exc = turn_exc
    finally:
        if holding:
            _lane_release(loop, acct)


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
        task = loop.create_task(coro, context=watcher.ctx)
        watcher.taskref(task)

    def _schedule(watcher):
        loop.call_soon_threadsafe(_run, cb(watcher), watcher, context=ctx)

    return _schedule
