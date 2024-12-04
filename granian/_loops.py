import asyncio
import os
import sys
from typing import Any, Callable, Dict, Iterable, List, Optional, Tuple


WrappableT = Callable[..., Any]
LoopBuilderT = Callable[..., asyncio.AbstractEventLoop]


class Registry:
    __slots__ = ['_data']

    def __init__(self):
        self._data: Dict[str, WrappableT] = {}

    def __contains__(self, key: str) -> bool:
        return key in self._data

    def keys(self) -> Iterable[str]:
        return self._data.keys()

    def register(self, key: str) -> Callable[[WrappableT], WrappableT]:
        def wrap(builder: WrappableT) -> WrappableT:
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
        self._data: Dict[str, Tuple[LoopBuilderT, Dict[str, Any]]] = {}

    def register(self, key: str, packages: Optional[List[str]] = None) -> Callable[[LoopBuilderT], LoopBuilderT]:
        packages = packages or []

        def wrap(builder: LoopBuilderT) -> LoopBuilderT:
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

    def get(self, key: str) -> asyncio.AbstractEventLoop:
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


@loops.register('rloop', packages=['rloop'])
def build_rloop(rloop):
    asyncio.set_event_loop_policy(rloop.EventLoopPolicy())
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    return loop


@loops.register('auto')
def build_auto_loop():
    if 'rloop' in loops:
        return loops.get('rloop')
    if 'uvloop' in loops:
        return loops.get('uvloop')
    return loops.get('asyncio')
