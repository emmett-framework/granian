<p align="center">
  <img width="550" src="https://emmett.sh/static/img/granian-logo-xb-fw.png" alt="granian">
</p>

<p align="center">
<em>The Rust HTTP server for Python</em>
</p>

---

Granian is a Rust HTTP server for Python applications built on top of [Hyper](https://github.com/hyperium/hyper) and [Tokio](https://github.com/tokio-rs/tokio).

## Rationale

The main reasons behind Granian design are:

- Have a single, correct HTTP implementation, supporting versions 1, 2 (and eventually 3)
- Provide a single package for several platforms
- Avoid the usual Gunicorn + uvicorn + http-tools dependency composition on unix systems
- Provide stable [performance](https://github.com/emmett-framework/granian/blob/master/benchmarks/vs.md) when compared to existing alternatives

Adopting Granian would thus be a good choice when:

- wanting a modern, single dependency to serve both ASGI and WSGI applications
- looking for the most performant way to serve your Python application under HTTP/2
- you need great concurrency capabilities, especially with websockets
- you care about throughput more than everything else

On the other hand, Granian won't be the ideal option if:

- you want a *pure Python* solution
- you need advanced debugging features
- your application relies on `trio` or `gevent`
- you're looking for ASGI extensions [not (yet) implemented](https://github.com/emmett-framework/granian/issues/93)

## Features

- Supports ASGI/3, [RSGI](https://github.com/emmett-framework/granian/blob/master/docs/spec/RSGI.md) and WSGI interface applications
- HTTP/1 and HTTP/2 protocols
- HTTPS and mTLS
- Websockets
- Direct static files serving
- ASGI [pathsend](https://asgi.readthedocs.io/en/latest/extensions.html#path-send) extension

## Quickstart

You can install Granian using pip:

    $ pip install granian

### ASGI

Create an application in your `main.py`:

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

and serve it using Granian CLI:

    $ granian --interface asgi main:app

### RSGI

Create an application your `main.py`:

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

and serve it using Granian CLI:

    $ granian --interface rsgi main:app

### WSGI

Create an application your `main.py`:

```python
def app(environ, start_response):
    start_response('200 OK', [('content-type', 'text/plain')])
    return [b"Hello, world!"]
```

and serve it using Granian CLI:

    $ granian --interface wsgi main:app

## Extra dependencies

Mind that Granian also provides several extra dependencies you might be interested into, in particular:

- dotenv (allows to load environment files)
- pname (allows to customize processes' names)
- reload (adds reload on changes functionality)
- [rloop](https://pypi.org/project/rloop/)
- [uvloop](https://pypi.org/project/uvloop/)
- [winloop](https://pypi.org/project/winloop/)

You can combine the above extras to suit your needs, eg:

    $ pip install granian[pname,uvloop]

For further information, check the options below.

## Options

You can check all the options provided by Granian with the `--help` command:

```
$ granian --help
Usage: granian [OPTIONS] APP

  APP  Application target to serve.  [required]

Options:
  --host TEXT                     Host address to bind to  [env var:
                                  GRANIAN_HOST; default: (127.0.0.1)]
  --port INTEGER                  Port to bind to.  [env var: GRANIAN_PORT;
                                  default: 8000]
  --uds PATH                      Unix Domain Socket to bind to.  [env var:
                                  GRANIAN_UDS]
  --uds-permissions OCTAL INTEGER
                                  Unix Domain Socket file permissions  [env
                                  var: GRANIAN_UDS_PERMISSIONS]
  --interface [asgi|asginl|rsgi|wsgi]
                                  Application interface type  [env var:
                                  GRANIAN_INTERFACE; default: (rsgi)]
  --http [auto|1|2]               HTTP version  [env var: GRANIAN_HTTP;
                                  default: (auto)]
  --ws / --no-ws                  Enable websockets handling  [env var:
                                  GRANIAN_WEBSOCKETS; default: (enabled)]
  --workers INTEGER RANGE         Number of worker processes  [env var:
                                  GRANIAN_WORKERS; default: 1; x>=1]
  --blocking-threads INTEGER RANGE
                                  Number of blocking threads (per worker)
                                  [env var: GRANIAN_BLOCKING_THREADS; x>=1]
  --blocking-threads-idle-timeout DURATION
                                  The maximum amount of time in seconds (or a
                                  human-readable duration) an idle blocking
                                  thread will be kept alive  [env var:
                                  GRANIAN_BLOCKING_THREADS_IDLE_TIMEOUT;
                                  default: 30; 10<=x<=600]
  --runtime-threads INTEGER RANGE
                                  Number of runtime threads (per worker)  [env
                                  var: GRANIAN_RUNTIME_THREADS; default: 1;
                                  x>=1]
  --runtime-blocking-threads INTEGER RANGE
                                  Number of runtime I/O blocking threads (per
                                  worker)  [env var:
                                  GRANIAN_RUNTIME_BLOCKING_THREADS; x>=1]
  --runtime-mode [auto|mt|st]     Runtime mode to use (single/multi threaded)
                                  [env var: GRANIAN_RUNTIME_MODE; default:
                                  (auto)]
  --loop [auto|asyncio|rloop|uvloop|winloop]
                                  Event loop implementation  [env var:
                                  GRANIAN_LOOP; default: (auto)]
  --task-impl [asyncio|rust]      Async task implementation to use  [env var:
                                  GRANIAN_TASK_IMPL; default: (asyncio)]
  --backlog INTEGER RANGE         Maximum number of connections to hold in
                                  backlog (globally)  [env var:
                                  GRANIAN_BACKLOG; default: 1024; x>=128]
  --backpressure INTEGER RANGE    Maximum number of requests to process
                                  concurrently (per worker)  [env var:
                                  GRANIAN_BACKPRESSURE; default:
                                  (backlog/workers); x>=1]
  --http1-buffer-size INTEGER RANGE
                                  Sets the maximum buffer size for HTTP/1
                                  connections  [env var:
                                  GRANIAN_HTTP1_BUFFER_SIZE; default: 417792;
                                  x>=8192]
  --http1-header-read-timeout INTEGER RANGE
                                  Sets a timeout (in milliseconds) to read
                                  headers  [env var:
                                  GRANIAN_HTTP1_HEADER_READ_TIMEOUT; default:
                                  30000; 1<=x<=60000]
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
  --http2-initial-connection-window-size INTEGER RANGE
                                  Sets the max connection-level flow control
                                  for HTTP2  [env var: GRANIAN_HTTP2_INITIAL_C
                                  ONNECTION_WINDOW_SIZE; default: 1048576;
                                  x>=1024]
  --http2-initial-stream-window-size INTEGER RANGE
                                  Sets the `SETTINGS_INITIAL_WINDOW_SIZE`
                                  option for HTTP2 stream-level flow control
                                  [env var:
                                  GRANIAN_HTTP2_INITIAL_STREAM_WINDOW_SIZE;
                                  default: 1048576; x>=1024]
  --http2-keep-alive-interval INTEGER RANGE
                                  Sets an interval (in milliseconds) for HTTP2
                                  Ping frames should be sent to keep a
                                  connection alive  [env var:
                                  GRANIAN_HTTP2_KEEP_ALIVE_INTERVAL;
                                  1<=x<=60000]
  --http2-keep-alive-timeout DURATION
                                  Sets a timeout (in seconds or a human-
                                  readable duration) for receiving an
                                  acknowledgement of the HTTP2 keep-alive ping
                                  [env var: GRANIAN_HTTP2_KEEP_ALIVE_TIMEOUT;
                                  default: 20; x>=1]
  --http2-max-concurrent-streams INTEGER RANGE
                                  Sets the SETTINGS_MAX_CONCURRENT_STREAMS
                                  option for HTTP2 connections  [env var:
                                  GRANIAN_HTTP2_MAX_CONCURRENT_STREAMS;
                                  default: 200; x>=10]
  --http2-max-frame-size INTEGER RANGE
                                  Sets the maximum frame size to use for HTTP2
                                  [env var: GRANIAN_HTTP2_MAX_FRAME_SIZE;
                                  default: 16384; x>=1024]
  --http2-max-headers-size INTEGER RANGE
                                  Sets the max size of received header frames
                                  [env var: GRANIAN_HTTP2_MAX_HEADERS_SIZE;
                                  default: 16777216; x>=1]
  --http2-max-send-buffer-size INTEGER RANGE
                                  Set the maximum write buffer size for each
                                  HTTP/2 stream  [env var:
                                  GRANIAN_HTTP2_MAX_SEND_BUFFER_SIZE; default:
                                  409600; x>=1024]
  --log / --no-log                Enable logging  [env var:
                                  GRANIAN_LOG_ENABLED; default: (enabled)]
  --log-level [critical|error|warning|warn|info|debug|notset]
                                  Log level  [env var: GRANIAN_LOG_LEVEL;
                                  default: (info)]
  --log-config FILE               Logging configuration file (json)  [env var:
                                  GRANIAN_LOG_CONFIG]
  --access-log / --no-access-log  Enable access log  [env var:
                                  GRANIAN_LOG_ACCESS_ENABLED; default:
                                  (disabled)]
  --access-log-fmt TEXT           Access log format  [env var:
                                  GRANIAN_LOG_ACCESS_FMT]
  --ssl-certificate FILE          SSL certificate file  [env var:
                                  GRANIAN_SSL_CERTIFICATE]
  --ssl-keyfile FILE              SSL key file  [env var: GRANIAN_SSL_KEYFILE]
  --ssl-keyfile-password TEXT     SSL key password  [env var:
                                  GRANIAN_SSL_KEYFILE_PASSWORD]
  --ssl-protocol-min [tls1.2|tls1.3]
                                  Set the minimum supported protocol for SSL
                                  connections.  [env var:
                                  GRANIAN_SSL_PROTOCOL_MIN; default: (tls1.3)]
  --ssl-ca FILE                   Root SSL cerificate file for client
                                  verification  [env var: GRANIAN_SSL_CA]
  --ssl-crl FILE                  SSL CRL file(s)  [env var: GRANIAN_SSL_CRL]
  --ssl-client-verify / --no-ssl-client-verify
                                  Verify clients SSL certificates  [env var:
                                  GRANIAN_SSL_CLIENT_VERIFY; default:
                                  (disabled)]
  --url-path-prefix TEXT          URL path prefix the app is mounted on  [env
                                  var: GRANIAN_URL_PATH_PREFIX]
  --respawn-failed-workers / --no-respawn-failed-workers
                                  Enable workers respawn on unexpected exit
                                  [env var: GRANIAN_RESPAWN_FAILED_WORKERS;
                                  default: (disabled)]
  --respawn-interval FLOAT        The number of seconds to sleep between
                                  workers respawn  [env var:
                                  GRANIAN_RESPAWN_INTERVAL; default: 3.5]
  --rss-sample-interval DURATION  The sample rate in seconds (or a human-
                                  readable duration) for the resource monitor
                                  [env var: GRANIAN_RSS_SAMPLE_INTERVAL;
                                  default: 30; 10<=x<=300]
  --rss-samples INTEGER RANGE     The number of consecutive samples to
                                  consider a worker over resource limit  [env
                                  var: GRANIAN_RSS_SAMPLES; default: 1; x>=1]
  --workers-lifetime DURATION     The maximum amount of time in seconds (or a
                                  human-readable duration) a worker will be
                                  kept alive before respawn  [env var:
                                  GRANIAN_WORKERS_LIFETIME; x>=60]
  --workers-max-rss INTEGER RANGE
                                  The maximum amount of memory (in MiB) a
                                  worker can consume before respawn  [env var:
                                  GRANIAN_WORKERS_MAX_RSS; x>=1]
  --workers-kill-timeout DURATION
                                  The amount of time in seconds (or a human-
                                  readable duration) to wait for killing
                                  workers that refused to gracefully stop
                                  [env var: GRANIAN_WORKERS_KILL_TIMEOUT;
                                  default: (disabled); 1<=x<=1800]
  --factory / --no-factory        Treat target as a factory function, that
                                  should be invoked to build the actual target
                                  [env var: GRANIAN_FACTORY; default:
                                  (disabled)]
  --working-dir DIRECTORY         Set the working directory  [env var:
                                  GRANIAN_WORKING_DIR]
  --env-files FILE                Environment file(s) to load (requires
                                  granian[dotenv] extra)  [env var:
                                  GRANIAN_ENV_FILES]
  --static-path-route TEXT        Route for static file serving  [env var:
                                  GRANIAN_STATIC_PATH_ROUTE; default:
                                  (/static)]
  --static-path-mount DIRECTORY   Path to mount for static file serving  [env
                                  var: GRANIAN_STATIC_PATH_MOUNT]
  --static-path-expires DURATION  Cache headers expiration (in seconds or a
                                  human-readable duration) for static file
                                  serving. 0 to disable.  [env var:
                                  GRANIAN_STATIC_PATH_EXPIRES; default: 86400;
                                  x>=0]
  --reload / --no-reload          Enable auto reload on application's files
                                  changes (requires granian[reload] extra)
                                  [env var: GRANIAN_RELOAD; default:
                                  (disabled)]
  --reload-paths PATH             Paths to watch for changes  [env var:
                                  GRANIAN_RELOAD_PATHS; default: (Working
                                  directory)]
  --reload-ignore-dirs TEXT       Names of directories to ignore changes for.
                                  Extends the default list of directories to
                                  ignore in watchfiles' default filter  [env
                                  var: GRANIAN_RELOAD_IGNORE_DIRS]
  --reload-ignore-patterns TEXT   File/directory name patterns (regex) to
                                  ignore changes for. Extends the default list
                                  of patterns to ignore in watchfiles' default
                                  filter  [env var:
                                  GRANIAN_RELOAD_IGNORE_PATTERNS]
  --reload-ignore-paths PATH      Absolute paths to ignore changes for  [env
                                  var: GRANIAN_RELOAD_IGNORE_PATHS]
  --reload-tick INTEGER RANGE     The tick frequency (in milliseconds) the
                                  reloader watch for changes  [env var:
                                  GRANIAN_RELOAD_TICK; default: 50;
                                  50<=x<=5000]
  --reload-ignore-worker-failure / --no-reload-ignore-worker-failure
                                  Ignore worker failures when auto reload is
                                  enabled  [env var:
                                  GRANIAN_RELOAD_IGNORE_WORKER_FAILURE;
                                  default: (disabled)]
  --process-name TEXT             Set a custom name for processes (requires
                                  granian[pname] extra)  [env var:
                                  GRANIAN_PROCESS_NAME]
  --pid-file FILE                 A path to write the PID file to  [env var:
                                  GRANIAN_PID_FILE]
  --version                       Show the version and exit.
  --help                          Show this message and exit.
```

### Human-readable durations

Whenever Granian accepts a duration, it can be specified either as sole number, in which case it is interpreted as a number of seconds, or using one of the following suffixes:

| suffix | meaning |
| --- | --- |
| `s` | seconds (same as no suffix) |
| `m` | minutes |
| `h` | hours |
| `d` | days |

### Logging

Despite being a Rust project, Granian is a good Python citizen and uses the standard library's [`logging`](https://docs.python.org/3/library/logging.html) module to produce logs. This means you can freely configure your logging level and format using the [standard idioms](https://docs.python.org/3/howto/logging.html) you probably familiar with.

As many other web servers, Granian uses two different loggers, specifically:

- the `_granian` logger for runtime messages
- the `granian.access` logger for access logs

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

### Workers and threads

Granian offers different options to configure the number of workers and threads to be run, in particular:

- **workers**: the total number of processes holding a dedicated Python interpreter that will run the application
- **blocking threads**: the number of threads per worker interacting with the Python interpreter
- **runtime threads**: the number of Rust threads per worker that will perform network I/O
- **runtime blocking threads**: the number of Rust threads per worker involved in blocking operations. The main role of these threads is dealing with blocking I/O – like file system operations.

In general, Granian will try its best to automatically pick proper values for the threading configuration, leaving to you the responsibility to choose the number of workers you need.    
There is no *golden rule* here, as these numbers will vastly depend both on your application behavior and the deployment target, but we can list some suggestions:
- matching the amount of CPU cores for the workers is generally the best starting point; on containerized environments like docker or k8s is best to have 1 worker per container though and scale your containers using the relevant orchestrator;
- the default number of **runtime threads** and **runtime blocking threads** is fine for the vast majority of applications out there; you might want to increase the first for applications dealing with several concurrently opened websockets or if you primarily use HTTP/2, and lowering the second only if you serve the same few files to a lot of connections;

In regards of blocking threads, the option is irrelevant on asynchronous protocols, as all the interop will happen with the AsyncIO event loop which will also be the one holding the GIL for the vast majority of the time, and thus the value is fixed to a single thread; on synchronous protocols like WSGI instead, it will be the maximum amount of threads interacting – and thus trying to acquire the GIL – with your application code. All those threads will be spawned on-demand depending on the amount of concurrency, and they'll be shutdown after the amount of inactivity time specified with the relevant setting.    
In general, and unless you have a very specific use-case to do so (for example, if your application have an average millisecond response, a very limited amount of blocking threads usually delivers better throughput) you should avoid to tune this threadpool size and configure a backpressure value that suits your needs instead. In that regards, please check the next section.

Also, **you should generally avoid to configure workers and threads based on numbers suggested for other servers**, as Granian architecture is quite different from projects like Gunicorn or Uvicorn.

### Backpressure

Since Granian runs a separated Rust runtime aside of your application that will handle I/O and "send work" to the Python interpreter, a mechanism to avoid pushing more work that what the Python interpreter can actually do is provided: backpressure.

Backpressure in Granian operates at the single worker's connections accept loop, practically interrupting the loop in case too many requests are waiting to be processed down the line. You can think of it as _a secondary backlog_, handled by Granian itself in addition to the network stack one provided by the OS kernel (and configured with apposite parameter).

While on asynchronous protocols, the default value for the backpressure should work fine for the vast majority of applications, as _work_ will be handled and suspended by the AsyncIO event loop, on synchronous protocols there's no way to predict the amount of interrupts (and thus GIL releases) your application would do on a single request, and thus you should configure a value that makes sense in your environment. For example, if your WSGI application never does I/O within a request-reponse flow, then you can't really go beyond serial, and thus any backpressure value above 2 wouldn't probably make any difference, as all the requests will just be waiting to acquire the GIL in order to be processed. On the other hand, if your application makes external network requests within the standard request-response flow, a large backpressure can help, as during the time spent on those code paths you can still process other requests. Another example would be if your application communicate with a database, and you have a limited amount of connections that can be opened to that database: in this case setting the backpressure to that value would definitely be the best option.

In general, think of backpressure as the maximum amount of concurrency you want to handle (per worker) in your application, after which Granian will halt and wait before pushing more work.

> **Warning**: since backpressure interacts with the accept loop, it will limit connections, not the single requests. Kept-alive connections will handle multiple requests within a single connection, but Granian won't count those requests in the *actual pressure*. This also means, if you typically have several long-running kept-alive connections to your service (for example, if you run behind a reverse proxy), a backpressure value can prevent Granian to accept new connections once the amount of kept-alive connections reaches that limit. Under this circumstances, you want to ensure the configured backpressure is higher than the expected amount of kept-alive connections, and if you're trying to limit the concurrency, it's probably better to configure the blocking threads number rather than the backpressure itself.

### Runtime mode

Granian offers two different runtime threading paradigms, due to the fact the runtime can be multi-threaded – in opposition to what happens in Python event-loop which can only run as a single thread.

Given you specify N threads with the relevant option, in **st** mode Granian will spawn N single-threaded Rust runtimes, while in **mt** mode Granian will spawn a single multi-threaded runtime with N threads.

Benchmarks suggests **st** mode to be more efficient with a small amount of processes, while **mt** mode seems to scale more efficiently where you have a large number of CPUs. Real performance will though depend on specific application code, and thus *your mileage might vary*.

### Proxies and forwarded headers

Since none of the supported applications protocols define a strategy for proxies' *forwarded headers*, Granian doesn't provide any option to configure its behaviour around them.

What Granian provides instead, for contexts in which is being run behind a reverse proxy, are *wrappers* you can use on top of your application, in order to alter the request scope based on the headers forwarded by the proxy itself. You can find such wrappers in the `granian.utils.proxies` module:

```python
from granian.utils.proxies import wrap_asgi_with_proxy_headers, wrap_wsgi_with_proxy_headers

async def my_asgi_app(scope, receive, send):
    ...

def my_wsgi_app(environ, start_response):
    ...

my_asgi_app = wrap_asgi_with_proxy_headers(my_asgi_app, trusted_hosts="1.2.3.4")
my_wsgi_app = wrap_wsgi_with_proxy_headers(my_wsgi_app, trusted_hosts="1.2.3.4")
```

With these wrappers, Granian will use:

- the [`X-Forwarded-For`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/X-Forwarded-For) header to determine the client's IP address 
- the [`X-Forwarded-Proto`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/X-Forwarded-Proto) header to determine the client's protocol

and replace the relevant request scope attributes with these values.

Since altering the request scope based on values from headers is security-sensitive, Granian will check the request is coming from a trusted host as specified by the `trusted_hosts` argument. By default this value is set to `127.0.0.1`, which means Granian will only intercept those headers if the proxy resides on the same machine, but most likely that's not the case in a production environment: you should thus provide the correct set of addresses to the wrappers.

The `trusted_hosts` argument accepts either a string or a list of strings, where valid values are IP addresses (for example, `192.0.2.1` or `fd12:3456:789a::1`) and CIDR ranges (for example, `192.0.2.0/24` or `2001:db8:abcd::/48`). The special *catch-all value* `"*"` (or `["*"]`) will make Granian trust all hosts and effectively disable the security check.

## Free-threaded Python

> **Warning:** free-threaded Python support is still experimental and highly discouraged in *production environments*.

Since version 2.0 Granian supports free-threaded Python. While the installation process remains the same, as wheels for the free-threaded version are published separately, here we list some key differences from the GIL version.

- Workers are threads instead of separated processes, so there will always be a single Python interpreter running
- The application is thus loaded a single time and shared between workers
- In asynchronous protocols like ASGI and RSGI each worker runs its own AsyncIO event loop like the GIL version, but the loop will run in the worker thread instead of the Python main thread

> **Note:** if for any reason the GIL gets enabled on the free-threaded build, Granian will refuse to start. This means you can't use the free-threaded build on GIL enabled interpreters.

While for asynchronous protocols nothing really changes in terms of workers and threads configuration, as the scaling will be still driven by the number of AsyncIO event loops running (so the same rules for GIL workers apply), on synchronous protocols like WSGI every GIL-related limitation is theoretically absent.    
While the general rules in terms of I/O-bound vs CPU-bound load still apply, at the time being there's not enough data to make suggestions in terms of workers and threads tuning in the free-threaded Python land, and thus you will need to experiment with those values depending on your specific workload.

## Customising Granian

Running Granian directly from Python instead of its CLI gives you access to some customization interfaces that let you alter its standard behaviour.

### AsyncIO event loop initialization

As soon as you run Granian directly from Python instead of its CLI, you can customise the default event loop initialisation policy by overwriting the `auto` policy. Let's say, for instance, you want to use the selector event loop on Windows:

```python
import asyncio
from granian import Granian, loops

@loops.register('auto')
def build_loop():
    asyncio.set_event_loop_policy(asyncio.WindowsSelectorEventLoopPolicy())
    return asyncio.new_event_loop()


Granian(...).serve()
```

### Hooks

Granian provides hooks registration interfaces to run code during specific phases of its lifecycle. Specifically, you have the following methods available:

- `on_startup`
- `on_shutdown`
- `on_reload`

The mentioned methods accept a callable with no arguments that will be invoked during the relevant lifecycle phases. You can register your hooks simply passing them to the relevant method:

```python
from granian import Granian

def my_hook():
    print("hello from reload!")

server = Granian(...)
server.on_reload(my_hook)
```

### Embedding Granian in your project

For projects requiring advance lifecycle management, or implementing their own process management strategy, Granian provides an *embeddable* server implementation, which provides async interfaces and won't spawn workers as processes or threads, but will run them as AsyncIO tasks.

> **Warning:** the embeddable server is still experimental.

> **Note:** the embeddable server only supports async protocols, thus WSGI is not supported. It's also limited to a single worker, as it runs over an existing event loop.

To embed Granian in your project, you can import the server from the relevant module:

```python
from granian.server.embed import Server

server = Server(my_app, interface="asgi")

async def my_main():
    await server.serve()
```

> **Note:** as you might already figured out, the embed server accepts the application object as its first argument, instead of the import target string of the standard servers.

Given the `serve` method is now async, the embeddable server also provides two methods to manage its lifecycle, specifically:

- `stop`
- `reload`

The idea is that you can spawn the server as a task, and later on interact with it in your own process loop:

```python
async def my_main():
    server_task = asyncio.create_task(server.serve())
    await my_logic()
    server.stop()
    await server_task
```

## Project status

Granian is being actively maintained and is compatible with Python 3.10 and above versions.

Granian follows a *semantic versioning* scheme for its releases, with a `{major}.{minor}.{patch}` scheme for versions numbers, where:

- *major* versions might introduce breaking changes
- *minor* versions introduce new features and backward compatible changes
- *patch* versions only introduce bug and security fixes

Mind that bug and security fixes are **usually provided for the latest minor version only**.

Granian is used *in production* by projects like [paperless-ngx](https://github.com/paperless-ngx/paperless-ngx/blob/v2.17.1/pyproject.toml#L81), [reflex](https://github.com/reflex-dev/reflex/blob/v0.7.14/pyproject.toml#L25), [searxng](https://github.com/searxng/searxng/blob/a0ff173799d978873ba022e5d5d8467b8ca7e822/requirements-server.txt#L1) and [Weblate](https://github.com/WeblateOrg/weblate/blob/weblate-5.13.3/pyproject.toml#L238), and by companies like [Microsoft](https://github.com/microsoft/call-center-ai/blob/958b3192020ab8a49cabd328a873eaa70e8865bc/pyproject.toml#L26), [Mozilla](https://github.com/mozilla/bedrock/blob/2025-06-25/requirements/prod.in#L33) and [Sentry](https://github.com/getsentry/sentry/blob/25.10.0/pyproject.toml#L38).

## License

Granian is released under the BSD License.
