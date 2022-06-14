from enum import Enum


class Interfaces(str, Enum):
    ASGI = "asgi"
    RSGI = "rsgi"


class ThreadModes(str, Enum):
    runtime = "runtime"
    workers = "workers"
