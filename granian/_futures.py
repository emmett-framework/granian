import asyncio


def future_wrapper(watcher, coro, handler):
    fut = asyncio.ensure_future(coro)
    fut.add_done_callback(handler(watcher))
