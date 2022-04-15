from typing import Any, Dict


class Receiver:
    async def __call__(self) -> bytes: ...


class Sender:
    def __call__(self, message: Dict[str, Any]) -> None: ...


class Scope:
    client: str
    headers: Dict[bytes, bytes]
    http_version: str
    method: str
    path: str
    proto: str
    query_string: str
    scheme: str
