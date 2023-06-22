# Granian

A Rust HTTP server for Python applications.

## Rationale

The main reasons behind Granian design are:

- Have a single, correct HTTP implementation, supporting versions 1, 2 (and eventually 3)
- Provide a single package for several platforms 
- Avoid the usual Gunicorn + uvicorn + http-tools dependency composition on unix systems
- Provide stable [performance](https://github.com/emmett-framework/granian/blob/master/benchmarks/README.md) when compared to existing alternatives

## Features

- Supports ASGI/3, [RSGI](https://github.com/emmett-framework/granian/blob/master/docs/spec/RSGI.md) and WSGI interface applications
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

## Options

You can check all the options provided by Granian with the `--help` command:

```shell
$ granian --help
Usage: granian [OPTIONS] APP

Arguments:
  APP  Application target to serve.  [required]

Options:
  --host TEXT                     Host address to bind to.  [default:
                                  127.0.0.1]
  --port INTEGER                  Port to bind to.  [default: 8000]
  --interface [asgi|rsgi|wsgi]    Application interface type.  [default: rsgi]
  --http [auto|1|2]               HTTP version.  [default: auto]
  --ws / --no-ws                  Enable websockets handling  [default:
                                  (enabled)]
  --workers INTEGER RANGE         Number of worker processes.  [default: 1;
                                  x>=1]
  --threads INTEGER RANGE         Number of threads.  [default: 1; x>=1]
  --threading-mode [runtime|workers]
                                  Threading mode to use.  [default: workers]
  --loop [auto|asyncio|uvloop]    Event loop implementation  [default: auto]
  --opt / --no-opt                Enable loop optimizations  [default:
                                  (disabled)]
  --backlog INTEGER RANGE         Maximum number of connections to hold in
                                  backlog.  [default: 1024; x>=128]
  --log-level [critical|error|warning|warn|info|debug]
                                  Log level  [default: info]
  --log-config FILE               Logging configuration file (json)
  --ssl-keyfile FILE              SSL key file
  --ssl-certificate FILE          SSL certificate file
  --url-path-prefix TEXT          URL path prefix the app is mounted on
  --reload / --no-reload          Enable auto reload on application's files
                                  changes  [default: no-reload]
  --version                       Shows the version and exit.
  --install-completion [bash|zsh|fish|powershell|pwsh]
                                  Install completion for the specified shell.
  --show-completion [bash|zsh|fish|powershell|pwsh]
                                  Show completion for the specified shell, to
                                  copy it or customize the installation.
  --help                          Show this message and exit.
```

### Threading mode

Granian offers two different threading paradigms, due to the fact the inner Rust runtime can be multi-threaded – in opposition to what happens in Python event-loop which can only run as a single thread.

Given you specify N threads with the relevant option, in **workers** threading mode Granian will spawn N single-threaded Rust runtimes, while in **runtime** threading mode Granian will spawn a single multi-threaded runtime with N threads.

Benchmarks suggests **workers** mode to be more efficient with a small amount of processes, while **runtime** mode seems to scale more efficiently where you have a large number of CPUs. Real performance will though depend on specific application code, and thus *your mileage might vary*.

### Event loop optimizations

With the `--opt` option Granian will use custom task handlers for Python coroutines and awaitables to improve Python code execution. Due to the nature of such handlers some libraries and specific application code relying on `asyncio` internals might not work.

You might test the effect such optimizations cause over your application and decide wether to enable 'em or leave 'em disabled (as per default).

## Project status

Granian is currently under active development.

Granian is compatible with Python 3.8 and above versions.

## License

Granian is released under the BSD License.
