import asyncio


def future_wrapper(coro, watcher):
    fut = asyncio.ensure_future(coro)
    fut.add_done_callback(future_handler(watcher))


def future_handler(watcher):
    def handler(task):
        try:
            task.result()
        except Exception:
            watcher.err()
            raise
        watcher.done()
    return handler
