from typing import Dict


class Scope:
    client: str
    headers: Dict[bytes, bytes]
    http_version: str
    method: str
    path: str
    proto: str
    query_string: str
    scheme: str
