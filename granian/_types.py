from typing import List, Optional, Tuple, Union


class WebsocketMessage:
    kind: int
    data: Union[bytes, str]


SSLCtx = Tuple[bool, Optional[str], Optional[str], Optional[str], Optional[str], List[str], bool]
