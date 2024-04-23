from typing import Any, Callable, TypeVar, Union


class WebsocketMessage:
    kind: int
    data: Union[bytes, str]


WrappableT = TypeVar('WrappableT', bound=Callable[..., Any])
