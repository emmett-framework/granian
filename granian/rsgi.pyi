from typing import Any, Dict, List


class Headers:
    def __contains__(self, key: str) -> bool: ...
    def keys(self) -> List[str]: ...
    def values(self) -> List[str]: ...
    def items(self) -> List[str]: ...
    def get(self, key: str, default: Any = None) -> Any: ...


class Scope:
    proto: str
    http_version: str
    client: str
    scheme: str
    method: str
    path: str
    query_string: str

    @property
    def headers(self) -> Headers: ...


class Receiver:
    async def __call__(self) -> bytes: ...
