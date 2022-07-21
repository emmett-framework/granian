from typing import Any, Dict, Union


class WebsocketMessage:
    kind: int
    data: Union[bytes, str]


class ASGIProtocol:
    async def receive(self) -> Dict[str, Any]: ...
    async def send(self, data: Dict[str, Any]): ...
