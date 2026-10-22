# Native routes

**Status:** experimental

## Abstract

This document describes *native routes*: paths that are served directly by the Rust side of
Granian, without dispatching the request to the Python application.

The main motivation is to allow the implementation of endpoints which would otherwise pay
the cost of a full round-trip through the Python interpreter (and the GIL, on the GIL-enabled
builds) for every single message, like websockets.

This document also covers the relationship between this feature and
[#83](https://github.com/emmett-framework/granian/issues/83) – *Expose Rust interfaces* – which
is the "long term" version of the same idea: exposing the Granian server as a Rust library,
so that *any* Rust code can serve *any* path.

## The problem

Granian's request handling pipeline is currently a single, straight line:

```
socket --> hyper --> WorkerSvc::call --> protocol handler (asgi/rsgi/wsgi) --> Python callback --> response
```

The only step of that pipeline that is *not* Python-bound is the static files one, which is
implemented in `src/files.rs` and short-circuits the pipeline in `WorkerSvc::call`
(`src/workers.rs`).

Everything else – including the websockets handshake, which is fully implemented in Rust
(`src/ws.rs`) – is only reachable *through* the Python application, because the protocol
handlers (`src/asgi/http.rs`, `src/rsgi/http.rs`, `src/wsgi/http.rs`) are the ones converting
the hyper request into a Python scope.

## The approach

The pipeline gets a new, optional step, placed *before* the protocol handlers:

```
socket --> hyper --> WorkerSvc::call --> native router --> native handler (Rust)  --> response
                                       |--> static files
                                       |--> protocol handler (asgi/rsgi/wsgi) --> Python callback --> response
```

The native router is a list of `(path prefix, handler)` pairs, matched on the request path
with segment boundaries (the same semantics used by the static files mounts), where the most
specific (longest) prefix wins.

A native handler is an `async` callable receiving the raw hyper request and a context object,
and returning a hyper response:

```rust
pub type NativeHandler =
    Arc<dyn Fn(HTTPRequest, NativeContext) -> BoxFuture<'static, HTTPResponse> + Send + Sync>;
```

The context carries the runtime handle (to spawn tasks), the local/remote addresses and
whether the connection is TLS protected.

### Serving websockets natively

The websockets handshake machinery is already fully implemented in Rust in `src/ws.rs`:

- `is_upgrade_request()` checks the request headers
- `upgrade_intent()` validates the handshake and returns both the `101` response builder and
  the `HyperWebsocket` future

A native websockets handler is therefore a matter of performing the handshake and spawning a
task handling the upgraded stream:

```rust
let mut req = req;
match upgrade_intent(&mut req, None) {
    Ok((builder, ws)) => {
        ctx.rt.spawn(async move {
            if let Ok(stream) = ws.await {
                // handle the websocket stream
            }
        });
        builder.body(empty_body()).unwrap()
    }
    Err(_) => response_400(),
}
```

The connection needs to be built with hyper's `with_upgrades()` for the upgrade to complete;
Granian enables it based on the `websockets` configuration flag, and it is enabled
automatically when any native websockets route is registered.

## Using it

The routes are registered by the code building the extension module:

```rust
granian::register_native_route("/ws", granian::NativeRouteKind::Websocket, handler);
```

Granian itself registers a couple of *reference* routes (`/native_view` and `/native_ws`),
used by the test suite as a reference implementation.

### Embedding Granian as a library

The package is built as both a `cdylib` and an `rlib`, so that it can be embedded in other
Rust projects. Crates doing so provide the `granian._granian` module themselves, and
register their own native routes:

```rust
use pyo3::prelude::*;

#[pyo3::pymodule(gil_used = false)]
fn _granian(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    granian::init_py_module(py, module)?;
    granian::register_native_route("/ws", granian::NativeRouteKind::Websocket, handler);
    Ok(())
}
```

The `python-module` feature (enabled by default) provides the module: it must be disabled
when the module is provided by the embedding crate, in order to avoid the `PyInit__granian`
symbol being defined twice.

A complete example is available in `examples/native-views`.

> **Note:** the dependency direction is the important detail here: Granian is a *library
> dependency* of the embedding crate, which is the artifact being built. Changing a handler
> recompiles the embedding crate only, Granian is not recompiled at all.

## Constraints

- **Worker scope.** The clients group of a websockets route is bound to the worker. In the
  multi-process setup each worker is a separate process, hence messages are *not* propagated
  across workers. Broadcasting to all the clients of a multi-worker setup requires an
  external broker. In the free-threaded (multi-threaded) setup the group is shared across
  the runtime threads of the same worker.
- **HTTP/2.** Protocol upgrades are an HTTP/1.1 feature. In `http=2` mode native websockets
  routes cannot work; in `http=auto` mode they work only over HTTP/1.1 connections. This is
  the same limitation the application websockets have.
- **Access log.** Native requests bypass the Python callback wrapper, hence they are not
  reported in the access log.
- **Metrics.** Native requests are counted in the `requests_handled` counter, but they have
  no dedicated counter yet.
- **URL path prefix.** The `url_path_prefix` option is applied by the Python callback
  wrapper, hence native routes are matched against the *raw* request path.
- **Static files.** Static files are served before the native routes, hence a static mount
  takes precedence over a native route matching the same path.

## Testing

`tests/test_native.py` covers the feature: echo, broadcast between two clients, rejection of
non-websockets requests on a native path, coexistence with the application websockets routes,
and the native HTTP view.

## Long term: exposing the Rust interfaces (#83)

The feature described in this document allows serving paths natively, but the native code has
to be *compiled into* the artifact serving them. Issue #83 asks for something more general:
exposing the Granian server as a Rust library, so that third party Rust projects (like
[Robyn](https://github.com/sansyrox/robyn)) can use the Granian server (and its protocols) to
serve *and* interact with Python code.

The current state is a partial implementation of it: the package is usable as a library, and
the native routes are the extension point. What is still missing is a *pure Rust* server API
(one that does not require the Python side at all), which implies moving the sockets, the
signals, the metrics and the request dispatch out of their Python types:

| Module | Python coupling |
| --- | --- |
| `http.rs`, `ws.rs`, `files.rs`, `tls.rs` | none |
| `net.rs` | `ListenerSpec`/`SocketHolder` are `pyclass`es wrapping pure Rust socket builders |
| `workers.rs` | `WorkerConfig` holds `Py<ListenerSpec>`, `Py<SocketHolder>`, `Py<MetricsAggregator>`; `WorkerCTX*` holds `Arc<Py<CallbackScheduler>>` |
| `serve.rs` | takes `Python` and `Bound<PyAny>` (event loop) and `Py<WorkerSignal>` |
| `runtime.rs` | `RuntimeRef` carries the Python event loop and exposes `spawn_blocking(\|py\| ...)` |
| `callbacks.rs`, `ipc.rs`, `metrics.rs`, `asgi/`, `rsgi/`, `wsgi/` | fully Python |

The proposed structure is a split of the package in two:

```
granian-core/     pure Rust: http, ws, files, tls, net, workers, serve, native
granian/          pyo3 bindings: protocols (asgi/rsgi/wsgi), callbacks, metrics, ipc
```

with the decoupling work being:

1. **`WorkerConfig`**: own the socket specification in the core (`ListenerSpec`,
   `SocketHolder` become plain Rust types, the `pyclass`es become thin wrappers over them),
   replace `Py<MetricsAggregator>` with a core `MetricsSink` trait, and replace
   `Py<WorkerSignal>` with a core shutdown signal.
2. **Request dispatch**: replace the `ArcCBScheduler` parameter of the protocol handlers with
   a core trait, e.g. `Arc<dyn RequestHandler>`, implemented by the pyo3 side on top of the
   callback scheduler. The `F: Fn(RuntimeRef, ArcCBScheduler, ...) -> Ret` signature of the
   target handler becomes `F: Fn(RuntimeRef, Arc<dyn RequestHandler>, ...) -> Ret`.
3. **`runtime.rs`**: split the pure tokio runtime handle from the Python specific bits (event
   loop, blocking runner, `spawn_blocking`).
4. **`serve.rs`**: make the `serve_*` functions independent from `Python`/`Bound<PyAny>`,
   taking the shutdown signal and the event loop (if any) as optional/core-level parameters.

The resulting public API would look like:

```rust
use granian_core::prelude::*;

let server = Server::builder()
    .listener(("127.0.0.1", 8000))
    .http(HttpMode::Auto)
    .websockets(true)
    .route("/ws", |req, ctx| Box::pin(async move { handle_ws(req, ctx).await }))
    .build()?;

server.serve().await?;
```

The sizing of that work is roughly 12 files and ~2000 lines, and it needs to preserve the
current behaviour of the Python facing server (the tests in `tests/` are the safety net). It
is a considerably bigger effort than the native routes feature, and it is the reason why the
issue is still open.

## References

- #83 – Expose Rust interfaces
- `src/native.rs` – native router and the reference native routes
- `src/workers.rs` – `WorkerSvc::call`, where the native router is consulted
- `src/ws.rs` – websockets handshake machinery
- `examples/native-views` – example of a crate embedding Granian as a library
