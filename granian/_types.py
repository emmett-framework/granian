from typing import Union


class WebsocketMessage:
    kind: int
    data: Union[bytes, str]
