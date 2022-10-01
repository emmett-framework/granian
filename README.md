# Granian

A Rust HTTP server for Python applications.

## Rationale

The main reasons behind Granian design are:

- Have a single, correct HTTP implementation, supporting versions 1, 2 (and eventually 3)
- Provide a single package for several platforms 
- Avoid the usual Gunicorn + uvicorn + http-tools dependency composition on unix systems
- Provide stable performance compared with existing alternatives

## Features

- Supports ASGI/3 and RSGI interface applications
- Implements HTTP/1 and HTTP/2 protocols
- Supports HTTPS
- Supports websockets over HTTP/1 and HTTP/2

## Project status

Granian is currently in early-stage development.

Granian is compatible with Python 3.7 and above versions on unix platforms and 3.8 and above on Windows.

## License

Granian is released under the BSD License.
