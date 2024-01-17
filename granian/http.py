from dataclasses import dataclass
from typing import Optional


@dataclass
class HTTP1Settings:
    keep_alive: bool = True
    max_buffer_size: int = 8192 + 4096 * 100
    pipeline_flush: bool = False


@dataclass
class HTTP2Settings:
    adaptive_window: bool = False
    initial_connection_window_size: int = 1024 * 1024
    initial_stream_window_size: int = 1024 * 1024
    keep_alive_interval: Optional[int] = None
    keep_alive_timeout: int = 20
    max_concurrent_streams: int = 200
    max_frame_size: int = 1024 * 16
    max_headers_size: int = 16 * 1024 * 1024
    max_send_buffer_size: int = 1024 * 400
