class WebsocketMessage:
    kind: int
    data: bytes | str


SSLCtx = tuple[bool, str | None, str | None, str | None, str, str | None, list[str], bool]
