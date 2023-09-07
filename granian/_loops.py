import asyncio
import os
import signal
import sys
from typing import Any, Callable, Dict, Iterable, List, Optional, Tuple


class Registry:
    __slots__ = ['_data']

    def __init__(self):
        self._data: Dict[str, Callable[..., Any]] = {}

    def __contains__(self, key: str) -> bool:
        return key in self._data

    def keys(self) -> Iterable[str]:
        return self._data.keys()

    def register(self, key: str) -> Callable[[], Callable[..., Any]]:
        def wrap(builder: Callable[..., Any]) -> Callable[..., Any]:
            self._data[key] = builder
            return builder

        return wrap

    def get(self, key: str) -> Callable[..., Any]:
        try:
            return self._data[key]
        except KeyError:
            raise RuntimeError(f"'{key}' implementation not available.")


class BuilderRegistry(Registry):
    __slots__ = []

    def __init__(self):
        self._data: Dict[str, Tuple[Callable[..., Any], List[str]]] = {}

    def register(self, key: str, packages: Optional[List[str]] = None) -> Callable[[], Callable[..., Any]]:
        packages = packages or []

        def wrap(builder: Callable[..., Any]) -> Callable[..., Any]:
            loaded_packages, implemented = {}, True
            try:
                for package in packages:
                    __import__(package)
                    loaded_packages[package] = sys.modules[package]
            except ImportError:
                implemented = False
            if implemented:
                self._data[key] = (builder, loaded_packages)
            return builder

        return wrap

    def get(self, key: str) -> Callable[..., Any]:
        try:
            builder, packages = self._data[key]
        except KeyError:
            raise RuntimeError(f"'{key}' implementation not available.")
        return builder(**packages)


loops = BuilderRegistry()


@loops.register('asyncio')
def build_asyncio_loop():
    loop = asyncio.new_event_loop() if os.name != 'nt' else asyncio.ProactorEventLoop()
    asyncio.set_event_loop(loop)
    return loop


@loops.register('uvloop', packages=['uvloop'])
def build_uv_loop(uvloop):
    asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    return loop


@loops.register('auto')
def build_auto_loop():
    if 'uvloop' in loops:
        return loops.get('uvloop')
    return loops.get('asyncio')


def set_loop_signals(loop, signals):
    signal_event = asyncio.Event()

    def signal_handler(signum, frame):
        signal_event.set()

    try:
        for sigval in signals:
            loop.add_signal_handler(sigval, signal_handler, sigval, None)
    except NotImplementedError:
        for sigval in signals:
            signal.signal(sigval, signal_handler)

    return signal_event
