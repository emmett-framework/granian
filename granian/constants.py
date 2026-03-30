from enum import Enum


class StrEnum(str, Enum):
    def __str__(self) -> str:
        return str(self.value)


class Interfaces(StrEnum):
    ASGI = 'asgi'
    ASGINL = 'asginl'
    RSGI = 'rsgi'
    RSGI2 = 'rsgi2'
    WSGI = 'wsgi'


class HTTPModes(StrEnum):
    auto = 'auto'
    http1 = '1'
    http2 = '2'


class RuntimeModes(StrEnum):
    auto = 'auto'
    mt = 'mt'
    st = 'st'


class PyRuntimes(StrEnum):
    asyncio = 'asyncio'
    tonio = 'tonio'
    threading = 'threading'


class Loops(StrEnum):
    auto = 'auto'
    asyncio = 'asyncio'
    rloop = 'rloop'
    uvloop = 'uvloop'
    winloop = 'winloop'


class TaskImpl(StrEnum):
    asyncio = 'asyncio'
    rust = 'rust'


class SSLProtocols(StrEnum):
    tls12 = 'tls1.2'
    tls13 = 'tls1.3'
