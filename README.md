# Granian

A Rust HTTP server for Python applications.

## Rationale

The main reasons behind Granian design are:

- Have a single, correct HTTP implementation, supporting versions 1, 2 (and eventually 3)
- Provide a single package for several platforms 
- Avoid the usual Gunicorn + uvicorn + http-tools dependency composition on unix systems
- Provide stable performance when compared to existing alternatives

## Features

- Supports ASGI/3 and [RSGI](https://github.com/emmett-framework/granian/blob/master/docs/spec/RSGI.md) interface applications
- Implements HTTP/1 and HTTP/2 protocols
- Supports HTTPS
- Supports Websockets over HTTP/1 and HTTP/2

## Quickstart

You can install Granian using pip:

    $ pip install granian

Create an ASGI application in your `main.py`:

```python
async def app(scope, receive, send):
    assert scope['type'] == 'http'

    await send({
        'type': 'http.response.start',
        'status': 200,
        'headers': [
            [b'content-type', b'text/plain'],
        ],
    })
    await send({
        'type': 'http.response.body',
        'body': b'Hello, world!',
    })
```

and serve it:

    $ granian --interface asgi main:app

You can also create an app using the [RSGI](https://github.com/emmett-framework/granian/blob/master/docs/spec/RSGI.md) specification:

```python
async def app(scope, proto):
    assert scope.proto == 'http'

    proto.response_str(
        status=200,
        headers=[
            ('content-type', 'text/plain')
        ],
        body="Hello, world!"
    )
```

and serve it using:

    $ granian --interface rsgi main:app

## Project status

Granian is currently in early-stage development.

Granian is compatible with Python 3.7 and above versions on unix platforms and 3.8 and above on Windows.

## License

Granian is released under the BSD License.
