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
- Supports Websockets

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

  APP  Application target to serve.  [required]

Options:
  --host TEXT                     Host address to bind to  [env var:
                                  GRANIAN_HOST; default: (127.0.0.1)]
  --port INTEGER                  Port to bind to.  [env var: GRANIAN_PORT;
                                  default: 8000]
  --interface [asgi|asginl|rsgi|wsgi]
                                  Application interface type  [env var:
                                  GRANIAN_INTERFACE; default: (rsgi)]
  --http [auto|1|2]               HTTP version  [env var: GRANIAN_HTTP;
                                  default: (auto)]
  --ws / --no-ws                  Enable websockets handling  [env var:
                                  GRANIAN_WEBSOCKETS; default: (enabled)]
  --workers INTEGER RANGE         Number of worker processes  [env var:
                                  GRANIAN_WORKERS; default: 1; x>=1]
  --threads INTEGER RANGE         Number of threads (per worker)  [env var:
                                  GRANIAN_THREADS; default: 1; x>=1]
  --blocking-threads INTEGER RANGE
                                  Number of blocking threads (per worker)
                                  [env var: GRANIAN_BLOCKING_THREADS; x>=1]
  --threading-mode [runtime|workers]
                                  Threading mode to use  [env var:
                                  GRANIAN_THREADING_MODE; default: (workers)]
  --loop [auto|asyncio|uvloop]    Event loop implementation  [env var:
                                  GRANIAN_LOOP; default: (auto)]
  --opt / --no-opt                Enable loop optimizations  [env var:
                                  GRANIAN_LOOP_OPT; default: (disabled)]
  --backlog INTEGER RANGE         Maximum number of connections to hold in
                                  backlog (globally)  [env var:
                                  GRANIAN_BACKLOG; default: 1024; x>=128]
  --backpressure INTEGER RANGE    Maximum number of requests to process
                                  concurrently (per worker)  [env var:
                                  GRANIAN_BACKPRESSURE; default:
                                  (backlog/workers); x>=1]
  --http1-buffer-size INTEGER RANGE
                                  Set the maximum buffer size for HTTP/1
                                  connections  [env var:
                                  GRANIAN_HTTP1_BUFFER_SIZE; default: 417792;
                                  x>=8192]
  --http1-keep-alive / --no-http1-keep-alive
                                  Enables or disables HTTP/1 keep-alive  [env
                                  var: GRANIAN_HTTP1_KEEP_ALIVE; default:
                                  (enabled)]
  --http1-pipeline-flush / --no-http1-pipeline-flush
                                  Aggregates HTTP/1 flushes to better support
                                  pipelined responses (experimental)  [env
                                  var: GRANIAN_HTTP1_PIPELINE_FLUSH; default:
                                  (disabled)]
  --http2-adaptive-window / --no-http2-adaptive-window
                                  Sets whether to use an adaptive flow control
                                  for HTTP2  [env var:
                                  GRANIAN_HTTP2_ADAPTIVE_WINDOW; default:
                                  (disabled)]
  --http2-initial-connection-window-size INTEGER
                                  Sets the max connection-level flow control
                                  for HTTP2  [env var: GRANIAN_HTTP2_INITIAL_C
                                  ONNECTION_WINDOW_SIZE; default: 1048576]
  --http2-initial-stream-window-size INTEGER
                                  Sets the `SETTINGS_INITIAL_WINDOW_SIZE`
                                  option for HTTP2 stream-level flow control
                                  [env var:
                                  GRANIAN_HTTP2_INITIAL_STREAM_WINDOW_SIZE;
                                  default: 1048576]
  --http2-keep-alive-interval INTEGER
                                  Sets an interval for HTTP2 Ping frames
                                  should be sent to keep a connection alive
                                  [env var: GRANIAN_HTTP2_KEEP_ALIVE_INTERVAL]
  --http2-keep-alive-timeout INTEGER
                                  Sets a timeout for receiving an
                                  acknowledgement of the HTTP2 keep-alive ping
                                  [env var: GRANIAN_HTTP2_KEEP_ALIVE_TIMEOUT;
                                  default: 20]
  --http2-max-concurrent-streams INTEGER
                                  Sets the SETTINGS_MAX_CONCURRENT_STREAMS
                                  option for HTTP2 connections  [env var:
                                  GRANIAN_HTTP2_MAX_CONCURRENT_STREAMS;
                                  default: 200]
  --http2-max-frame-size INTEGER  Sets the maximum frame size to use for HTTP2
                                  [env var: GRANIAN_HTTP2_MAX_FRAME_SIZE;
                                  default: 16384]
  --http2-max-headers-size INTEGER
                                  Sets the max size of received header frames
                                  [env var: GRANIAN_HTTP2_MAX_HEADERS_SIZE;
                                  default: 16777216]
  --http2-max-send-buffer-size INTEGER
                                  Set the maximum write buffer size for each
                                  HTTP/2 stream  [env var:
                                  GRANIAN_HTTP2_MAX_SEND_BUFFER_SIZE; default:
                                  409600]
  --log / --no-log                Enable logging  [env var:
                                  GRANIAN_LOG_ENABLED; default: (enabled)]
  --log-level [critical|error|warning|warn|info|debug]
                                  Log level  [env var: GRANIAN_LOG_LEVEL;
                                  default: (info)]
  --log-config FILE               Logging configuration file (json)  [env var:
                                  GRANIAN_LOG_CONFIG]
  --access-log / --no-access-log  Enable access log  [env var:
                                  GRANIAN_LOG_ACCESS_ENABLED; default:
                                  (disabled)]
  --access-log-fmt TEXT           Access log format  [env var:
                                  GRANIAN_LOG_ACCESS_FMT]
  --ssl-keyfile FILE              SSL key file  [env var: GRANIAN_SSL_KEYFILE]
  --ssl-certificate FILE          SSL certificate file  [env var:
                                  GRANIAN_SSL_CERTIFICATE]
  --url-path-prefix TEXT          URL path prefix the app is mounted on  [env
                                  var: GRANIAN_URL_PATH_PREFIX]
  --respawn-failed-workers / --no-respawn-failed-workers
                                  Enable workers respawn on unexpected exit
                                  [env var: GRANIAN_RESPAWN_FAILED_WORKERS;
                                  default: (disabled)]
  --respawn-interval FLOAT        The number of seconds to sleep between
                                  workers respawn  [env var:
                                  GRANIAN_RESPAWN_INTERVAL; default: 3.5]
  --reload / --no-reload          Enable auto reload on application's files
                                  changes (requires granian[reload] extra)
                                  [env var: GRANIAN_RELOAD; default:
                                  (disabled)]
  --process-name TEXT             Set a custom name for processes (requires
                                  granian[pname] extra)  [env var:
                                  GRANIAN_PROCESS_NAME]
  --version                       Show the version and exit.
  --help                          Show this message and exit.
```

### Access log format

The access log format can be configured by specifying the atoms (see below) to include in a specific format. By default Granian will use `[%(time)s] %(addr)s - "%(method)s %(path)s %(protocol)s" %(status)d %(dt_ms).3f` as the format.

#### Access log atoms

The following atoms are available for use:

| identifier | description |
| --- | --- |
| addr | Client remote address |
| time | Datetime of the request | 
| dt_ms | Request duration in ms |
| status | HTTP response status |
| path | Request path (without query string) |
| query\_string | Request query string |
| method | Request HTTP method |
| scheme | Request scheme |
| protocol | HTTP protocol version |

### Processes and threads

Granian offers different options to configure the number of processes and threads to be run, in particular:

- **workers**: the total number of processes holding a dedicated Python interpreter that will run the application
- **threads**: the number of Rust threads per worker that will perform network I/O
- **blocking threads**: the number of Rust threads per worker involved in blocking operations. The main role of these threads is to deal with blocking I/O – like opening files – but on synchronous protocols like WSGI these threads will also be responsible of interacting with the application code.

In general, Granian will try its best to automatically pick proper values for the threading configuration, leaving to you the responsibility to choose the number of workers you need.    
There is no *golden rule* here, as these numbers will vastly depend both on your application behavior and the deployment target, but we can list some suggestions:
- matching the amount of CPU cores for the workers is generally the best starting point; on containerized environments like docker or k8s is best to have 1 worker per container though and scale your containers using the relevant orchestrator;
- the default number of threads is fine for the vast majority of applications out there; you might want to increase this number for applications dealing with several concurrently opened websockets; 
- the default number of blocking threads should work properly with the majority of applications; in synchronous protocols like WSGI this will also impact the number of concurrent requests you can handle, but you should use the `backpressure` configuration parameter to control it and set a lower number of blocking threads only if your application has a very low (1ms order) average response time;

Also, you should generally avoid to configure workers and threads based on numbers of other servers, as Granian architecture is quite different from projects like Gunicorn or Uvicorn.

### Threading mode

Granian offers two different threading paradigms, due to the fact the inner Rust runtime can be multi-threaded – in opposition to what happens in Python event-loop which can only run as a single thread.

Given you specify N threads with the relevant option, in **workers** threading mode Granian will spawn N single-threaded Rust runtimes, while in **runtime** threading mode Granian will spawn a single multi-threaded runtime with N threads.

Benchmarks suggests **workers** mode to be more efficient with a small amount of processes, while **runtime** mode seems to scale more efficiently where you have a large number of CPUs. Real performance will though depend on specific application code, and thus *your mileage might vary*.

### Event loop optimizations

With the `--opt` option Granian will use custom task handlers for Python coroutines and awaitables to improve Python code execution. Due to the nature of such handlers some libraries and specific application code relying on `asyncio` internals might not work.

You might test the effect such optimizations cause over your application and decide whether to enable 'em or leave 'em disabled (as per default).

## Project status

Granian is currently under active development.

Granian is compatible with Python 3.8 and above versions.

## License

Granian is released under the BSD License.
