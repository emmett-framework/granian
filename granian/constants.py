from enum import Enum


class StrEnum(str, Enum):
    def __str__(self) -> str:
        return str(self.value)


class Interfaces(StrEnum):
    ASGI = 'asgi'
    ASGINL = 'asginl'
    RSGI = 'rsgi'
    WSGI = 'wsgi'


class HTTPModes(StrEnum):
    auto = 'auto'
    http1 = '1'
    http2 = '2'


class ThreadModes(StrEnum):
    runtime = 'runtime'
    workers = 'workers'


class Loops(StrEnum):
    auto = 'auto'
    asyncio = 'asyncio'
    uvloop = 'uvloop'
