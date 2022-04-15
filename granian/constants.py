from enum import Enum


class Interfaces(str, Enum):
    ASGI = "asgi"
    RSGI = "rsgi"
