# RSGI Specification

**Version:** 1.3

## Abstract

This document proposes a standard interface between Rust network protocol servers (particularly web servers) and Python applications, intended to allow the handling of multiple common protocol styles (including HTTP, HTTP/2, and WebSocket).

This base specification is intended to fix in place the set of APIs by which these servers interact and run application code; each supported protocol (such as HTTP) has a sub-specification that outlines how to handle it in a specific way.

## Rationale

The ASGI specification works well in a *Python-only world*, allowing the same great flexibility WSGI introduced. However, its design is irrevocably tied to the Python language itself, and the AsyncIO implementation. For instance, ASGI design is built around the idea that the socket transport and the threading paradigm is handled by Python itself; a condition that might lead to inefficient paradigms when looking at implementation coming from different languages. We can summarise this concept into this phrase: *ASGI expects the lower protocol to be handled by Python*.

As the abstract suggests, RSGI is designed to solve the inefficiencies we described for servers written in the Rust language, where the actual I/O communication and threading components are handled outside the Python interpreter, to allow applications to take advantage of the performance provided by the protocol implementation.

RSGI attempts to preserve a simple application interface like ASGI does, while providing an abstraction that allows data to be sent and received through Rust built protocols. This is why, for example, RSGI keeps the same interfaces on the application layer both for HTTP requests and Websockets, but expects different usage of those interfaces based on the protocols: unlike ASGI, requests won't be handled using *messages*.

As we said, RSGI is not built around the idea of Python handling the lower protocols, and thus its design is not meant to preserve interoperability with ASGI and WSGI: the I/O fundamentals changed, and supporting the previous one would have been a flawed decision since its begin.

Its primary goal is to provide a way to write HTTP/1, HTTP/2, HTTP/3 and WebSocket code in Python, taking advantage of an efficient lower protocol.

## Overview

RSGI consists of two different components:

- A protocol server, which terminates sockets and translates them into connections and per-connection objects.
- An application, which lives inside a protocol server, is called once per connection, and handles connections events as they happen, emitting its own events back when necessary.

Like ASGI, the server hosts the application inside it, and dispatches incoming requests to it in a standardized format; like ASGI applications are asynchronous callables, and they communicate with the server by interacting with awaitable objects. RSGI applications must run as `async`/`await` compatible coroutines (i.e. `asyncio`-compatible) (on the main thread; they are free to use threading or other processes if they need synchronous code).

There are two separate parts to an RSGI connection:

- A *connection scope*, like ASGI, which represents a protocol connection to a user and survives until the connection closes.
- A *connection protocol* interface the application can interact with, that will responsible of trasmitting data from and to the client.

Applications are consequently called and awaited with a connection scope and a connection protocol to interact with. All this happening in an asynchronous event loop.

Each call of the application callable maps to a single incoming “socket” or connection, and is expected to last the lifetime of that connection plus a little longer if there is cleanup to do.

## Applications

RSGI applications should be a single async callable:

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
    authority: Optional[str]
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
- `authority`: an optional string containing the relevant pseudo-header (empty on HTTP versions prior to 2)

#### HTTP protocol interface

HTTP protocol object implements two awaitable methods to receive the request body, and five different methods to send data, in particular:

- `__call__` to receive the entire body in `bytes` format
- `__aiter__` to receive the body in `bytes` chunks
- `response_empty` to send back an empty response
- `response_str` to send back a response with a `str` body
- `response_bytes` to send back a response with `bytes` body
- `response_file` to send back a file response (from its path)
- `response_stream` to start a stream response

All the upper-mentioned response methods accepts an integer `status` parameter, a list of string tuples for the `headers` parameter, and the relevant typed `body` parameter (if applicable):

```
coroutine __call__() -> body
asynciterator __aiter__() -> body chunks
function response_empty(status, headers)
function response_str(status, headers, body)
function response_bytes(status, headers, body)
function response_file(status, headers, file)
function response_stream(status, headers) -> transport
```

The `response_stream` method will return a *transport object*, which implements the async messaging interfaces, specifically:

- a `send_bytes` awaitable method to produce outgoing messages from `bytes` content
- a `send_str` awaitable method to produce outgoing messages from `str` content

```
coroutine send_bytes(bytes)
coroutine send_str(str)
```

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
    authority: Optional[str]
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
- `authority`: an optional string containing the relevant pseudo-header (empty on HTTP versions prior to 2)

#### Websocket protocol interface

Websocket protocol object implements two interface methods for applications:

- the `accept` awaitable method
- the `close` method

```
coroutine accept() -> transport
function close(status)
```

The `accept` awaitable method will return a *transport object*, which implements the async messaging interfaces, specifically:

- a `receive` awaitable method which returns a single incoming message
- a `send_bytes` awaitable method to produce outgoing messages from `bytes` content
- a `send_str` awaitable method to produce outgoing messages from `str` content

```
coroutine receive() -> message
coroutine send_bytes(bytes)
coroutine send_str(str)
```

In RSGI websockets' incoming messages consist of objects with the form:

```python
class WebsocketMessage:
    kind: int
    data: Optional[Union[bytes, str]]
```

where `kind` is an integer with the following values:

| value | description |
| --- | --- |
| 0 | Websocket closed by client |
| 1 | Bytes message |
| 2 | String message |
