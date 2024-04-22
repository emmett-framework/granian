from typing import Optional, Union


class WebsocketMessage:
    kind: int
    data: Union[bytes, str]


class HTTP1ParamType:
    keep_alive: bool
    max_buffer_size: int
    pipeline_flush: bool


class HTTP2ParamType:
    adaptive_window: bool
    initial_connection_window_size: int
    initial_stream_window_size: int
    keep_alive_interval: Optional[int]
    keep_alive_timeout: int
    max_concurrent_streams: int
    max_frame_size: int
    max_headers_size: int
    max_send_buffer_size: int
