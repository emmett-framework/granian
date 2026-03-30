import threading


def _wrap_runtime_hook(server, method):
    def _wrapper():
        return method(server)

    return _wrapper


def _tonio_runtime_async_pre(server):
    import tonio.colored as tonio

    ev = tonio.Event()

    def _thread():
        async def _run():
            await ev.wait()

        tonio.run(_run(), threads=server.pyruntime_threads, context=True)

    server._pyruntime_event = ev
    th = threading.Thread(target=_thread)
    th.start()


def _tonio_runtime_async_post(server):
    server._pyruntime_event.set()
