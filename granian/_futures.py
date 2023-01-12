import asyncio

async def task_wrapper(task, watcher):
    try:
        await task
    except Exception:
        watcher.err()
        raise
    watcher.done()

def future_wrapper(coro, watcher):
    return asyncio.create_task(task_wrapper(coro, watcher))
