from typing import Any, List, Tuple


class Headers:
    def __contains__(self, key: str) -> bool: ...
    def keys(self) -> List[str]: ...
    def values(self) -> List[str]: ...
    def items(self) -> List[Tuple[str]]: ...
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
