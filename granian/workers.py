import asyncio
import contextvars
import signal

import uvloop
from granian._granian import ASGIWorker as GranianASGIWorker
from granian._loops import set_loop_signals
from granian.asgi import _callback_wrapper
from gunicorn.workers.base import Worker


class ASGIWorker(Worker):
    async def notify_task(self) -> None:
        while True:
            self.notify()
            await asyncio.sleep(self.timeout)

    def run(self) -> None:
        uvloop.install()
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        shutdown_event = set_loop_signals(loop, [signal.SIGTERM, signal.SIGINT])
        worker = GranianASGIWorker(
            worker_id=self.pid,
            socket_fd=self.sockets[0].fileno(),
            threads=1,
            pthreads=1,
            http_mode="1",
            http1_buffer_max=65536,
            websockets_enabled=True,
            ssl_enabled=False,
        )
        loop.create_task(self.notify_task())
        worker.serve_wth(
            _callback_wrapper(self.wsgi),
            loop,
            contextvars.copy_context(),
            shutdown_event.wait(),
        )
