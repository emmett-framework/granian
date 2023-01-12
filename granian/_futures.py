async def future_watcher_wrapper(fut, watcher):
    try:
        await fut
    except Exception:
        watcher.err()
        raise
    watcher.done()


def future_with_watcher(coro, watcher):
    return watcher.event_loop.create_task(future_watcher_wrapper(coro, watcher))
