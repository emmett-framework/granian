from enum import Enum


class Interfaces(str, Enum):
    ASGI = "asgi"
    RSGI = "rsgi"


class HTTPModes(str, Enum):
    auto = "auto"
    http1 = "1"
    http2 = "2"


class ThreadModes(str, Enum):
    runtime = "runtime"
    workers = "workers"


class Loops(str, Enum):
    auto = "auto"
    asyncio = "asyncio"
    uvloop = "uvloop"
