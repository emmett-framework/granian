# RSGI Specification

**Version:** 1.0

## Abstract

This document proposes a standard interface between Rust network protocol servers (particularly web servers) and Python applications, intended to allow handling of multiple common protocol styles (including HTTP, HTTP/2, and WebSocket).

This base specification is intended to fix in place the set of APIs by which these servers interact and run application code; each supported protocol (such as HTTP) has a sub-specification that outlines how to handle it in a specific way.

## Overview

RSGI consists of two different components:

- A protocol server, which terminates sockets and translates them into connections and per-connection objects.
- An application, which lives inside a protocol server, is called once per connection, and handles connections events as they happen, emitting its own events back when necessary.

Like ASGI, the server hosts the application inside it, and dispatches incoming requests to it in a standardized format; like ASGI applications are asynchronous callables, and they communicate with the server by interacting with awaitable objects. RSGI applications must run as `async`/`await` compatible coroutines (i.e. `asyncio`-compatible) (on the main thread; they are free to use threading or other processes if they need synchronous code).

There are two separate parts to an RSGI connection:

- A *connection scope*, like ASGI, which represents a protocol connection to a user and survives until the connection closes.
- A *connection protocol* interface the application can interact with, that will responsible of trasmitting data from and to the client.

Applications are consequentially called and awaited with a connection scope and a connection protocol to interact with. All this happening in an asynchronous event loop.

Each call of the application callable maps to a single incoming “socket” or connection, and is expected to last the lifetime of that connection plus a little longer if there is cleanup to do.

## Applications

ASGI applications should be a single async callable:

```
coroutine application(scope, protocol)
```

- `scope`: the connection scope information, an object that contains type key specifying the protocol that is incoming and the relevant information
- `protocol`: an object with awaitable methods to communicate data

The application is called once per "connection". The definition of a connection and its lifespan are dictated by the protocol specification in question. For example, with HTTP it is one request, whereas for a WebSocket it is a single WebSocket connection.

The protocol-specific sub-specifications cover scope and protocol specifications.

## Protocols

### HTTP protocol

The HTTP format covers HTTP/1.0, HTTP/1.1 and HTTP/2. The HTTP version is available as a string in the scope.

#### HTTP connection scope

HTTP connections have a single-request connection scope - that is, your application will be called at the start of the request, and will last until the end of that specific request, even if the underlying socket is still open and serving multiple requests.

If you hold a response open for long-polling or similar, the connection scope will persist until the response closes from either the client or server side.

The scope object for HTTP protocol is defined as follows:

```python
class Scope:
    proto: Literal['http'] = 'http'
    rsgi_version: str
    http_version: str
    server: str
    client: str
    scheme: str
    method: str
    path: str
    query_string: str
    headers: Mapping[str, str]
```

And here are descriptions for the upper attributes:

- `rsgi_version`: a string containing the version of the RSGI spec
- `http_version`: a string containing the HTTP version (one of "1", "1.1" or "2")
- `server`: a string in the format `{address}:{port}`, where host is the listening address for this server, and port is the integer listening port
- `client`: a string in the format `{address}:{port}`, where host is the remote host's address and port is the remote port
- `scheme`: URL scheme portion (one of "http" or "https")
- `method`: the HTTP method name, uppercased
- `path`: HTTP request target excluding any query string
- `query_string`: URL portion after the `?`
- `headers`: a mapping-like object, where keys is the header name, and value is the header value

#### HTTP protocol interface

HTTP protocol object implements just a single awaitable method on `__call__`, which returns the request body in `bytes` format.

#### HTTP protocol responses

HTTP protocol expects a `namedtuple` object to be returned from applications once the request handling is completed and a response should be sent back to clients. For clarity reasons, we represent this `namedtuple` as a class with attributes:

```python
class Response:
    mode: int
    status: int
    headers: List[Tuple[str, str]]
    bytes_data: Optional[bytes]
    string_data: Optional[str]
    file_path: Optional[str]
```

The `status` item represents the HTTP status code and `mode` defines the response type within the following options:

| value | response type |
| --- | --- |
| 0 | Empty response |
| 1 | Bytes response |
| 2 | String response |
| 10 | Filepath response |

RSGI applications are responsible to accordingly fill `bytes_data`, `string_data` and `file_path` fields with `mode` value.

### Websocket protocol

WebSockets share some HTTP details - they have a path and headers - but also have more state. Again, most of that state is in the scope, which will live as long as the socket does.

#### Websocket connection scope

WebSocket connections' scope lives as long as the socket itself - if the application dies the socket should be closed, and vice-versa.

The scope object for Websocket protocol is defined as follows:

```python
class Scope:
    proto: Literal['ws'] = 'ws'
    rsgi_version: str
    http_version: str
    server: str
    client: str
    scheme: str
    method: str
    path: str
    query_string: str
    headers: Mapping[str, str]
```

And here are descriptions for the upper attributes:

- `rsgi_version`: a string containing the version of the RSGI spec
- `http_version`: a string containing the HTTP version (one of "1", "1.1" or "2")
- `server`: a string in the format `{address}:{port}`, where host is the listening address for this server, and port is the integer listening port
- `client`: a string in the format `{address}:{port}`, where host is the remote host's address and port is the remote port
- `scheme`: URL scheme portion (one of "http" or "https")
- `method`: the HTTP method name, uppercased
- `path`: HTTP request target excluding any query string
- `query_string`: URL portion after the `?`
- `headers`: a mapping-like object, where keys is the header name, and value is the header value

#### Websocket protocol interface

Websocket protocol object implements two interface methods for applications:

- the `accept` awaitable method
- the `close` method

The `accept` awaitable method will return a *transport object*, which implements the async messaging interfaces, specifically:

- a `receive` awaitable method which returns a single incoming message
- a `send_bytes` awaitable method to produce outgoing messages from `bytes` content
- a `send_str` awaitable method to produce outgoing messages from `str` content

In RSGI websockets' incoming messages consist of objects with the form:

```python
class WebsocketMessage:
    kind: int
    data: Optional[Union[bytes, str]]
```

where `kind` is an integer with the following values:

| value | response type |
| --- | --- |
| 0 | Websocket closed by client |
| 1 | Bytes message |
| 2 | String message |

#### Websocket protocol responses

RSGI applications should always return the result of protocol `close` method.
