"""RFC 8441 (websockets over HTTP/2) support.

The client side of the extended CONNECT method is implemented here, as no
Python library exposes it: the `h2` package is used to drive the HTTP/2
connection, and the websockets framing is handled on top of the h2 stream.
"""

import os
import socket
import struct

import pytest
from h2.config import H2Configuration
from h2.connection import H2Connection
from h2.events import DataReceived, ResponseReceived, StreamEnded

from granian.constants import HTTPModes

OPCODE_TEXT = 0x1
OPCODE_BINARY = 0x2
OPCODE_CLOSE = 0x8


def _client_frame(payload: bytes, opcode: int = OPCODE_TEXT) -> bytes:
    """Build a masked client frame (RFC 6455 section 5.3)."""
    mask = os.urandom(4)
    masked = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
    length = len(payload)

    header = bytes([0x80 | opcode])
    if length < 126:
        header += bytes([0x80 | length])
    elif length < 65536:
        header += bytes([0x80 | 126]) + struct.pack('>H', length)
    else:
        header += bytes([0x80 | 127]) + struct.pack('>Q', length)

    return header + mask + masked


def _server_frames(buffer: bytes) -> tuple[list[bytes], bytes]:
    """Extract the complete server frames out of a buffer."""
    frames = []
    while len(buffer) >= 2:
        length = buffer[1] & 0x7F
        offset = 2
        if length == 126:
            if len(buffer) < 4:
                break
            length = struct.unpack('>H', buffer[2:4])[0]
            offset = 4
        elif length == 127:
            if len(buffer) < 10:
                break
            length = struct.unpack('>Q', buffer[2:10])[0]
            offset = 10
        if len(buffer) < offset + length:
            break
        frames.append(buffer[offset : offset + length])
        buffer = buffer[offset + length :]

    return frames, buffer


class H2WebSocketClient:
    """A minimal RFC 8441 websockets client."""

    def __init__(self, host: str, port: int, path: str):
        self.host = host
        self.port = port
        self.path = path
        self.conn = H2Connection(config=H2Configuration(client_side=True, header_encoding=None))
        self.sock = socket.create_connection((host, port))
        self.buffer = b''
        self.stream_id = None

    def __enter__(self):
        self.conn.initiate_connection()
        self.sock.sendall(self.conn.data_to_send())
        return self

    def __exit__(self, *args):
        self.sock.close()

    def connect(self) -> int:
        self.stream_id = self.conn.get_next_available_stream_id()
        self.conn.send_headers(
            self.stream_id,
            [
                (':method', 'CONNECT'),
                (':protocol', 'websocket'),
                (':scheme', 'http'),
                (':path', self.path),
                (':authority', f'{self.host}:{self.port}'),
                ('sec-websocket-version', '13'),
            ],
        )
        self.sock.sendall(self.conn.data_to_send())

        while True:
            event = self._next_event()
            if isinstance(event, ResponseReceived):
                if event.stream_id != self.stream_id:
                    continue
                return int(dict(event.headers)[b':status'])

    def send(self, payload: bytes, opcode: int = OPCODE_TEXT) -> None:
        self.conn.send_data(self.stream_id, _client_frame(payload, opcode), end_stream=False)
        self.sock.sendall(self.conn.data_to_send())

    def recv(self, timeout: float = 5.0) -> bytes:
        self.sock.settimeout(timeout)
        while True:
            frames, self.buffer = _server_frames(self.buffer)
            if frames:
                return frames[0]
            event = self._next_event()
            if isinstance(event, DataReceived):
                self.buffer += event.data
            elif isinstance(event, StreamEnded):
                raise ConnectionError('stream ended')

    def _next_event(self):
        while True:
            data = self.sock.recv(65536)
            if not data:
                raise ConnectionError('connection closed')
            for event in self.conn.receive_data(data):
                out = self.conn.data_to_send()
                if out:
                    self.sock.sendall(out)
                return event


@pytest.mark.asyncio
@pytest.mark.parametrize('server', ['asgi', 'rsgi'], indirect=True)
@pytest.mark.parametrize('runtime_mode', ['mt', 'st'])
async def test_app_ws_over_h2(server, runtime_mode):
    async with server(runtime_mode, http=HTTPModes.http2) as port:
        with H2WebSocketClient('127.0.0.1', port, '/ws_echo') as client:
            status = client.connect()
            assert status == 200

            client.send(b'foo')
            assert client.recv() == b'foo'
