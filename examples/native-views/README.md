# Granian native views

An example of a crate embedding [Granian](https://github.com/emmett-framework/granian) as a
Rust library in order to serve *native routes*: paths handled directly by the Rust side,
without ever reaching the Python application.

The crate builds a Python extension module named `granian._granian`, which **replaces** the
one shipped with the Granian package: the Python side is entirely stock Granian, while the
native routes are provided here.

## Usage

The extension module must be built against the same Python interpreter the Granian package
is installed for. In a working copy, the dependencies are usually installed in a local
virtual environment (the `.venv` directory created by `uv`, which is not tracked by the
repository):

```bash
# build the extension module
make build PYTHON=.venv/bin/python

# install it over the stock Granian one
make install PYTHON=.venv/bin/python
```

When the environment is already the active one, the default (`python3`) is enough:

```bash
make build
make install
```

The extension module is then served by the stock Granian server:

```bash
granian --interface rsgi --port 8000 package:app
```

```
$ curl http://127.0.0.1:8000/native_view
{"path":"/native_view","query":"","remote":"127.0.0.1:55568","secure":false,"source":"native-views"}
```

## How it works

The crate depends on Granian as a library and provides the `granian._granian` module itself:

```rust
use pyo3::prelude::*;

#[pyo3::pymodule(gil_used = false)]
fn _granian(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    // Initialise the Granian module: this provides the workers, the callback
    // scheduler and everything else the Python side expects to find.
    granian::init_py_module(py, module)?;

    // Register the native routes: they are served by this crate, without ever
    // reaching the Python application.
    granian::register_native_route(
        "/native_view",
        granian::NativeRouteKind::Http,
        views::info_view,
    );
    granian::register_native_route(
        "/native_ws",
        granian::NativeRouteKind::Websocket,
        consumers::broadcast_consumer(),
    );

    Ok(())
}
```

Since the routes are registered by this crate, the dependency direction is the one that
matters: **Granian is a library dependency, this crate is the artifact**. Changing a handler
here recompiles this crate only, Granian is not recompiled at all.

> **Note:** the `default-features = false` in the `Cargo.toml` is required: the
> `python-module` feature provides the `granian._granian` module, which is provided by this
> crate instead.

## Handlers

A handler is an async callable receiving the request and a context, and returning the
response:

```rust
use futures::future::BoxFuture;
use granian::{HTTPRequest, HTTPResponse, NativeContext};

pub fn info_view(req: HTTPRequest, ctx: NativeContext) -> BoxFuture<'static, HTTPResponse> {
    Box::pin(async move {
        // ...
    })
}
```

The websockets handshake is performed by Granian, the handler is responsible for the
upgraded stream:

```rust
let mut req = req;
match granian::ws::upgrade_intent(&mut req, None) {
    Ok((builder, ws)) => {
        ctx.rt.spawn(async move {
            if let Ok(stream) = ws.await {
                // handle the stream
            }
        });
        builder.body(granian::http::empty_body()).unwrap()
    }
    Err(_) => bad_request(),
}
```
