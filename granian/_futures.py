from asyncio.tasks import _enter_task, _leave_task

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

    def __init__(self, loop, ctx, cb):
        super().__init__()
        self._schedule_fn = _cbsched_schedule(loop, ctx, self._run, cb)

    def _waker(self, coro):
        def _wake(fut):
            self._resume(coro, fut)

        return _wake

    def _resume(self, coro, fut):
        try:
            fut.result()
        except BaseException as exc:
            self._throw(coro, exc)
        else:
            self._run(coro)

    def _run(self, coro):
        _enter_task(self._loop, self)
        try:
            try:
                result = coro.send(None)
            except (KeyboardInterrupt, SystemExit):
                raise
            except BaseException:
                pass
            else:
                if getattr(result, '_asyncio_future_blocking', None):
                    result._asyncio_future_blocking = False
                    result.add_done_callback(self._waker(coro), context=self._ctx)
                elif result is None:
                    self._loop.call_soon(self._run, coro, context=self._ctx)
        finally:
            _leave_task(self._loop, self)

    def _throw(self, coro, exc):
        _enter_task(self._loop, self)
        try:
            coro.throw(exc)
        except BaseException:
            pass
        finally:
            _leave_task(self._loop, self)


def _cbsched_schedule(loop, ctx, run, cb):
    def _schedule(watcher):
        loop.call_soon_threadsafe(run, cb(watcher), context=ctx)

    return _schedule
