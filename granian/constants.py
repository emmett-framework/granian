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


class RuntimeModes(StrEnum):
    mt = 'mt'
    st = 'st'


class Loops(StrEnum):
    auto = 'auto'
    asyncio = 'asyncio'
    rloop = 'rloop'
    uvloop = 'uvloop'


class TaskImpl(StrEnum):
    asyncio = 'asyncio'
    rust = 'rust'
