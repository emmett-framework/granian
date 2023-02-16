
# Copyright 2023 Ciro Spaciari
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all
# copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
# SOFTWARE.


from asyncio import (
    base_futures,
    base_tasks,
    format_helpers,
    exceptions,
    futures,
    _register_task,
    # _enter_task,
    # current_task,
    # _leave_task,
    _unregister_task,
)
import contextvars
import sys
import inspect
import platform

is_pypy = platform.python_implementation() == "PyPy"

# GenericAlias
if sys.version_info >= (3, 9):
    GenericAlias = type(list[int])
else:
    GenericAlias = None

isfuture = base_futures.isfuture


_PENDING = base_futures._PENDING
_CANCELLED = base_futures._CANCELLED
_FINISHED = base_futures._FINISHED


class RequestTask:
    """This class is *almost* compatible with concurrent.futures.Future.
    Differences:
    - This class do not support current_task
    - This class executes the first step like node.js Promise
    - This class is not thread-safe.
    - result() and exception() do not take a timeout argument and
      raise an exception when the future isn't done yet.
    - Callbacks registered with add_done_callback() are always called
      via the event loop's call_soon().
    - This class is not compatible with the wait() and as_completed()
      methods in the concurrent.futures package.
    (In Python 3.4 or later we may be able to unify the implementations.)
    """

    # Class variables serving as defaults for instance variables.
    _state = _PENDING
    _result = None
    _exception = None
    _loop = None
    _source_traceback = None
    _cancel_message = None
    # A saved CancelledError for later chaining as an exception context.
    _cancelled_exc = None

    # This field is used for a dual purpose:
    # - Its presence is a marker to declare that a class implements
    #   the Future protocol (i.e. is intended to be duck-type compatible).
    #   The value must also be not-None, to enable a subclass to declare
    #   that it is not compatible by setting this to None.
    # - It is set by __iter__() below so that Task._step() can tell
    #   the difference between
    #   `await Future()` or`yield from Future()` (correct) vs.
    #   `yield Future()` (incorrect).
    _asyncio_future_blocking = False

    __log_traceback = False

    """A coroutine wrapped in a Future."""
    # An important invariant maintained while a Task not done:
    #
    # - Either _fut_waiter is None, and _step() is scheduled;
    # - or _fut_waiter is some Future, and _step() is *not* scheduled.
    #
    # The only transition from the latter to the former is through
    # _wakeup().  When _fut_waiter is not None, one of its callbacks
    # must be _wakeup().

    # If False, don't log a message if the task is destroyed whereas its
    # status is still pending
    _log_destroy_pending = True

    def __init__(
        self, coro, loop, default_done_callback=None, no_register=False, context=None
    ):
        """Initialize the future.
        The optional event_loop argument allows explicitly setting the event
        loop object used by the future. If it's not provided, the future uses
        the default event loop.
        """
        self._loop = loop
        self._name = "socketify.py-request-task"  # fixed name for compatibility
        self._context = context if context else contextvars.copy_context()

        if default_done_callback:
            self._callbacks = [(default_done_callback, self._context)]
        else:
            self._callbacks = []

        self._num_cancels_requested = 0
        self._must_cancel = False
        self._fut_waiter = None
        self._coro = coro
        if not no_register:
            self._log_destroy_pending = False
            if self._loop.get_debug():
                self._source_traceback = format_helpers.extract_stack(sys._getframe(1))
            _register_task(self)
            # if current_task():
            #     self._loop.call_soon(self.__step, context=self._context)
            # else:
            self.__step()
                

    def _reuse(self, coro, loop, default_done_callback=None):
        """Reuse an future that is not pending anymore."""
        self._state = _PENDING
        self._result = None
        self._exception = None
        self._source_traceback = None
        self._cancel_message = None
        self._cancelled_exc = None
        self._asyncio_future_blocking = False
        self._log_traceback = False
        _unregister_task(self)

        self._loop = loop
        self._name = "socketify.py-request-task"  # fixed name for compatibility
        self._context = contextvars.copy_context()

        if default_done_callback:
            self._callbacks = [(default_done_callback, self._context)]
        else:
            self._callbacks = []

        if self._loop.get_debug():
            self._source_traceback = format_helpers.extract_stack(sys._getframe(1))

        self._num_cancels_requested = 0
        self._must_cancel = False
        self._fut_waiter = None
        self._coro = coro

        _register_task(self)
        # if current_task():
        #     self._loop.call_soon(self.__step, context=self._context)
        # else:
        self.__step()
        

    def __repr__(self):
        return base_tasks._task_repr(self)

    def __del__(self):

        if self._state == _PENDING and self._log_destroy_pending and self._loop:
            context = {
                "task": self,
                "message": "Task was destroyed but it is pending!",
            }
            if self._source_traceback:
                context["source_traceback"] = self._source_traceback
            self._loop.call_exception_handler(context)
        if not self.__log_traceback:
            # set_exception() was not called, or result() or exception()
            # has consumed the exception
            return
        exc = self._exception
        context = {
            "message": f"{self.__class__.__name__} exception was never retrieved",
            "exception": exc,
            "future": self,
        }
        if self._source_traceback:
            context["source_traceback"] = self._source_traceback
        self._loop.call_exception_handler(context)

    __class_getitem__ = classmethod(GenericAlias)

    def get_coro(self):
        return self._coro

    def get_context(self):
        return self._context

    def get_name(self):
        return self._name

    def set_name(self, value):
        self._name = str(value)

    def set_result(self, result):
        raise RuntimeError("Task does not support set_result operation")

    def set_exception(self, exception):
        raise RuntimeError("Task does not support set_exception operation")

    def get_stack(self, *, limit=None):
        """Return the list of stack frames for this task's coroutine.
        If the coroutine is not done, this returns the stack where it is
        suspended.  If the coroutine has completed successfully or was
        cancelled, this returns an empty list.  If the coroutine was
        terminated by an exception, this returns the list of traceback
        frames.
        The frames are always ordered from oldest to newest.
        The optional limit gives the maximum number of frames to
        return; by default all available frames are returned.  Its
        meaning differs depending on whether a stack or a traceback is
        returned: the newest frames of a stack are returned, but the
        oldest frames of a traceback are returned.  (This matches the
        behavior of the traceback module.)
        For reasons beyond our control, only one stack frame is
        returned for a suspended coroutine.
        """
        return base_tasks._task_get_stack(self, limit)

    def print_stack(self, *, limit=None, file=None):
        """Print the stack or traceback for this task's coroutine.
        This produces output similar to that of the traceback module,
        for the frames retrieved by get_stack().  The limit argument
        is passed to get_stack().  The file argument is an I/O stream
        to which the output is written; by default output is written
        to sys.stderr.
        """
        return base_tasks._task_print_stack(self, limit, file)

    @property
    def _log_traceback(self):
        return self.__log_traceback

    @_log_traceback.setter
    def _log_traceback(self, val):
        if val:
            raise ValueError("_log_traceback can only be set to False")
        self.__log_traceback = False

    def get_loop(self):
        """Return the event loop the Future is bound to."""
        loop = self._loop
        if loop is None:
            raise RuntimeError("Future object is not initialized.")
        return loop

    def _make_cancelled_error(self):
        """Create the CancelledError to raise if the Future is cancelled.
        This should only be called once when handling a cancellation since
        it erases the saved context exception value.
        """
        if self._cancelled_exc is not None:
            exc = self._cancelled_exc
            self._cancelled_exc = None
            return exc

        if self._cancel_message is None:
            exc = exceptions.CancelledError()
        else:
            exc = exceptions.CancelledError(self._cancel_message)
        exc.__context__ = self._cancelled_exc
        # Remove the reference since we don't need this anymore.
        self._cancelled_exc = None
        return exc

    def cancel(self, msg=None):
        """Request that this task cancel itself.
        This arranges for a CancelledError to be thrown into the
        wrapped coroutine on the next cycle through the event loop.
        The coroutine then has a chance to clean up or even deny
        the request using try/except/finally.
        Unlike Future.cancel, this does not guarantee that the
        task will be cancelled: the exception might be caught and
        acted upon, delaying cancellation of the task or preventing
        cancellation completely.  The task may also return a value or
        raise a different exception.
        Immediately after this method is called, Task.cancelled() will
        not return True (unless the task was already cancelled).  A
        task will be marked as cancelled when the wrapped coroutine
        terminates with a CancelledError exception (even if cancel()
        was not called).
        This also increases the task's count of cancellation requests.
        """
        self._log_traceback = False
        if self.done():
            return False
        self._num_cancels_requested += 1
        # These two lines are controversial.  See discussion starting at
        # https://github.com/python/cpython/pull/31394#issuecomment-1053545331
        # Also remember that this is duplicated in _asynciomodule.c.
        # if self._num_cancels_requested > 1:
        #     return False
        if self._fut_waiter is not None:
            if self._fut_waiter.cancel(msg=msg):
                # Leave self._fut_waiter; it may be a Task that
                # catches and ignores the cancellation so we may have
                # to cancel it again later.
                return True
        # It must be the case that self.__step is already scheduled.
        self._must_cancel = True
        self._cancel_message = msg
        return True

    def _cancel(self, msg=None):
        """Cancel the future and schedule callbacks.
        If the future is already done or cancelled, return False.  Otherwise,
        change the future's state to cancelled, schedule the callbacks and
        return True.
        """
        self.__log_traceback = False
        if self._state != _PENDING:
            return False
        self._state = _CANCELLED
        self._cancel_message = msg
        self.__schedule_callbacks()
        return True

    def cancelling(self):
        """Return the count of the task's cancellation requests.
        This count is incremented when .cancel() is called
        and may be decremented using .uncancel().
        """
        return self._num_cancels_requested

    def uncancel(self):
        """Decrement the task's count of cancellation requests.
        This should be called by the party that called `cancel()` on the task
        beforehand.
        Returns the remaining number of cancellation requests.
        """
        if self._num_cancels_requested > 0:
            self._num_cancels_requested -= 1
        return self._num_cancels_requested

    def __schedule_callbacks(self):
        """Internal: Ask the event loop to call all callbacks.
        The callbacks are scheduled to be called as soon as possible. Also
        clears the callback list.
        """
        callbacks = self._callbacks[:]
        if not callbacks:
            return

        self._callbacks[:] = []
        for callback, ctx in callbacks:
            self._loop.call_soon(callback, self, context=ctx)

    def cancelled(self):
        """Return True if the future was cancelled."""
        return self._state == _CANCELLED

    # Don't implement running(); see http://bugs.python.org/issue18699

    def done(self):
        """Return True if the future is done.
        Done means either that a result / exception are available, or that the
        future was cancelled.
        """
        return self._state != _PENDING

    def result(self):
        """Return the result this future represents.
        If the future has been cancelled, raises CancelledError.  If the
        future's result isn't yet available, raises InvalidStateError.  If
        the future is done and has an exception set, this exception is raised.
        """
        if self._state == _CANCELLED:
            exc = self._make_cancelled_error()
            raise exc
        if self._state != _FINISHED:
            raise exceptions.InvalidStateError("Result is not ready.")
        self.__log_traceback = False
        if self._exception is not None:
            raise self._exception.with_traceback(self._exception_tb)
        return self._result

    def exception(self):
        """Return the exception that was set on this future.
        The exception (or None if no exception was set) is returned only if
        the future is done.  If the future has been cancelled, raises
        CancelledError.  If the future isn't done yet, raises
        InvalidStateError.
        """
        if self._state == _CANCELLED:
            exc = self._make_cancelled_error()
            raise exc
        if self._state != _FINISHED:
            raise exceptions.InvalidStateError("Exception is not set.")
        self.__log_traceback = False
        return self._exception

    def add_done_callback(self, fn, *, context=None):
        """Add a callback to be run when the future becomes done.
        The callback is called with a single argument - the future object. If
        the future is already done when this is called, the callback is
        scheduled with call_soon.
        """
        if self._state != _PENDING:
            self._loop.call_soon(fn, self, context=context)
        else:
            if context is None:
                context = contextvars.copy_context()
            self._callbacks.append((fn, context))

    # New method not in PEP 3148.

    def remove_done_callback(self, fn):
        """Remove all instances of a callback from the "call when done" list.
        Returns the number of callbacks removed.
        """
        filtered_callbacks = [(f, ctx) for (f, ctx) in self._callbacks if f != fn]
        removed_count = len(self._callbacks) - len(filtered_callbacks)
        if removed_count:
            self._callbacks[:] = filtered_callbacks
        return removed_count

    # So-called internal methods (note: no set_running_or_notify_cancel()).

    def _set_result(self, result):
        """Mark the future done and set its result.
        If the future is already done when this method is called, raises
        InvalidStateError.
        """
        if self._state != _PENDING:
            raise exceptions.InvalidStateError(f"{self._state}: {self!r}")
        self._result = result
        self._state = _FINISHED
        self.__schedule_callbacks()

    def _set_exception(self, exception):
        """Mark the future done and set an exception.
        If the future is already done when this method is called, raises
        InvalidStateError.
        """
        if self._state != _PENDING:
            raise exceptions.InvalidStateError(f"{self._state}: {self!r}")
        if isinstance(exception, type):
            exception = exception()
        if type(exception) is StopIteration:
            raise TypeError(
                "StopIteration interacts badly with generators "
                "and cannot be raised into a Future"
            )
        self._exception = exception
        self._exception_tb = exception.__traceback__
        self._state = _FINISHED
        self.__schedule_callbacks()
        self.__log_traceback = True

    def __await__(self):
        if not self.done():
            self._asyncio_future_blocking = True
            yield self  # This tells Task to wait for completion.
        if not self.done():
            raise RuntimeError("await wasn't used with future")
        return self.result()  # May raise too.

    def __step(self, exc=None):
        if self.done():
            raise exceptions.InvalidStateError(
                f"_step(): already done: {self!r}, {exc!r}"
            )
        if self._must_cancel:
            if not isinstance(exc, exceptions.CancelledError):
                exc = self._make_cancelled_error()
            self._must_cancel = False
        coro = self._coro
        self._fut_waiter = None
        # _enter_task(self._loop, self)
        # Call either coro.throw(exc) or coro.send(None).
        try:
            if exc is None:
                # We use the `send` method directly, because coroutines
                # don't have `__iter__` and `__next__` methods.
                result = coro.send(None)
            else:
                result = coro.throw(exc)
        except StopIteration as exc:
            if self._must_cancel:
                # Task is cancelled right before coro stops.
                self._must_cancel = False
                self._cancel(msg=self._cancel_message)
            else:
                self._set_result(exc.value)
        except exceptions.CancelledError as exc:
            # Save the original exception so we can chain it later.
            self._cancelled_exc = exc
            self._cancel()  # I.e., Future.cancel(self).
        except (KeyboardInterrupt, SystemExit) as exc:
            self._set_exception(exc)
            raise
        except BaseException as exc:
            self._set_exception(exc)
        else:
            blocking = getattr(result, "_asyncio_future_blocking", None)
            if blocking is not None:
                # Yielded Future must come from Future.__iter__().
                if futures._get_loop(result) is not self._loop:
                    new_exc = RuntimeError(
                        f"Task {self!r} got Future "
                        f"{result!r} attached to a different loop"
                    )
                    self._loop.call_soon(self.__step, new_exc, context=self._context)
                elif blocking:
                    if result is self:
                        new_exc = RuntimeError(f"Task cannot await on itself: {self!r}")
                        self._loop.call_soon(
                            self.__step, new_exc, context=self._context
                        )
                    else:
                        result._asyncio_future_blocking = False
                        result.add_done_callback(self.__wakeup, context=self._context)
                        self._fut_waiter = result
                        if self._must_cancel:
                            if self._fut_waiter.cancel(msg=self._cancel_message):
                                self._must_cancel = False
                else:
                    new_exc = RuntimeError(
                        f"yield was used instead of yield from "
                        f"in task {self!r} with {result!r}"
                    )
                    self._loop.call_soon(self.__step, new_exc, context=self._context)

            elif result is None:
                # Bare yield relinquishes control for one event loop iteration.
                self._loop.call_soon(self.__step, context=self._context)
            elif inspect.isgenerator(result):
                # Yielding a generator is just wrong.
                new_exc = RuntimeError(
                    f"yield was used instead of yield from for "
                    f"generator in task {self!r} with {result!r}"
                )
                self._loop.call_soon(self.__step, new_exc, context=self._context)
            else:
                # Yielding something else is an error.
                new_exc = RuntimeError(f"Task got bad yield: {result!r}")
                self._loop.call_soon(self.__step, new_exc, context=self._context)
        finally:
            # _leave_task(self._loop, self)
            self = None  # Needed to break cycles when an exception occurs.

    def __wakeup(self, future):
        try:
            future.result()
        except BaseException as exc:
            # This may also be a cancellation.
            self.__step(exc)
        else:
            # Don't pass the value of `future.result()` explicitly,
            # as `Future.__iter__` and `Future.__await__` don't need it.
            # If we call `_step(value, None)` instead of `_step()`,
            # Python eval loop would use `.send(value)` method call,
            # instead of `__next__()`, which is slower for futures
            # that return non-generator iterators from their `__iter__`.
            self.__step()
        self = None  # Needed to break cycles when an exception occurs.

    __iter__ = __await__  # make compatible with 'yield from'.


async def factory_task_wrapper(task, dispose):
    try:
        await task
    finally:
        dispose()

# This is only worthed for PyPy not CPython
class TaskFactory:
    def __init__(self, task_factory_max_items=100_000):
        self.items = []
        for _ in range(0, task_factory_max_items):
            task = RequestTask(None, None, None, True)
            if task._source_traceback:
                del task._source_traceback[-1]
            self.items.append(task)

    def __call__(self, loop, coro):
        if len(self.items) == 0:
            return create_task(loop, coro)
        task = self.items.pop()

        task._reuse(factory_task_wrapper(coro, lambda : self.items.append(task)), loop)
        return task


def create_task(loop, coro, default_done_callback=None, context=None):
    """Schedule a coroutine object.
    Return a task object.
    """
    task = RequestTask(coro, loop, default_done_callback, context=context)
    if task._source_traceback:
        del task._source_traceback[-1]
    return task
