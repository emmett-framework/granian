# PyO3 Asyncio

[![Actions Status](https://github.com/awestlake87/pyo3-asyncio/workflows/CI/badge.svg)](https://github.com/awestlake87/pyo3-asyncio/actions)
[![codecov](https://codecov.io/gh/awestlake87/pyo3-asyncio/branch/master/graph/badge.svg)](https://codecov.io/gh/awestlake87/pyo3-asyncio)
[![crates.io](https://img.shields.io/crates/v/pyo3-asyncio)](https://crates.io/crates/pyo3-asyncio)
[![minimum rustc 1.48](https://img.shields.io/badge/rustc-1.48+-blue.svg)](https://rust-lang.github.io/rfcs/2495-min-rust-version.html)

[Rust](http://www.rust-lang.org/) bindings for [Python](https://www.python.org/)'s [Asyncio Library](https://docs.python.org/3/library/asyncio.html). This crate facilitates interactions between Rust Futures and Python Coroutines and manages the lifecycle of their corresponding event loops.

- PyO3 Project: [Homepage](https://pyo3.rs/) | [GitHub](https://github.com/PyO3/pyo3)

- PyO3 Asyncio API Documentation: [stable](https://docs.rs/pyo3-asyncio/) | [master](https://awestlake87.github.io/pyo3-asyncio/master/doc)

- Guide for Async / Await [stable](https://pyo3.rs/v0.13.2/ecosystem/async-await.html) | [main](https://pyo3.rs/main/ecosystem/async-await.html)

- Contributing Notes: [github](https://github.com/awestlake87/pyo3-asyncio/blob/master/Contributing.md)

> PyO3 Asyncio is a _brand new_ part of the broader PyO3 ecosystem. Feel free to open any issues for feature requests or bugfixes for this crate.

**If you're a new-comer, the best way to get started is to read through the primer below! For `v0.13` and `v0.14` users I highly recommend reading through the [migration section](#migration-guide) to get a general idea of what's changed in `v0.14` and `v0.15`.**

## Usage

Like PyO3, PyO3 Asyncio supports the following software versions:

- Python 3.7 and up (CPython and PyPy)
- Rust 1.48 and up

## PyO3 Asyncio Primer

If you are working with a Python library that makes use of async functions or wish to provide
Python bindings for an async Rust library, [`pyo3-asyncio`](https://github.com/awestlake87/pyo3-asyncio)
likely has the tools you need. It provides conversions between async functions in both Python and
Rust and was designed with first-class support for popular Rust runtimes such as
[`tokio`](https://tokio.rs/) and [`async-std`](https://async.rs/). In addition, all async Python
code runs on the default `asyncio` event loop, so `pyo3-asyncio` should work just fine with existing
Python libraries.

In the following sections, we'll give a general overview of `pyo3-asyncio` explaining how to call
async Python functions with PyO3, how to call async Rust functions from Python, and how to configure
your codebase to manage the runtimes of both.

### Quickstart

Here are some examples to get you started right away! A more detailed breakdown
of the concepts in these examples can be found in the following sections.

#### Rust Applications

Here we initialize the runtime, import Python's `asyncio` library and run the given future to completion using Python's default `EventLoop` and `async-std`. Inside the future, we convert `asyncio` sleep into a Rust future and await it.

```toml
# Cargo.toml dependencies
[dependencies]
pyo3 = { version = "0.16" }
pyo3-asyncio = { version = "0.16", features = ["attributes", "async-std-runtime"] }
async-std = "1.9"
```

```rust
//! main.rs

use pyo3::prelude::*;

#[pyo3_asyncio::async_std::main]
async fn main() -> PyResult<()> {
    let fut = Python::with_gil(|py| {
        let asyncio = py.import("asyncio")?;
        // convert asyncio.sleep into a Rust Future
        pyo3_asyncio::async_std::into_future(asyncio.call_method1("sleep", (1.into_py(py),))?)
    })?;

    fut.await?;

    Ok(())
}
```

The same application can be written to use `tokio` instead using the `#[pyo3_asyncio::tokio::main]`
attribute.

```toml
# Cargo.toml dependencies
[dependencies]
pyo3 = { version = "0.16" }
pyo3-asyncio = { version = "0.16", features = ["attributes", "tokio-runtime"] }
tokio = "1.9"
```

```rust
//! main.rs

use pyo3::prelude::*;

#[pyo3_asyncio::tokio::main]
async fn main() -> PyResult<()> {
    let fut = Python::with_gil(|py| {
        let asyncio = py.import("asyncio")?;
        // convert asyncio.sleep into a Rust Future
        pyo3_asyncio::tokio::into_future(asyncio.call_method1("sleep", (1.into_py(py),))?)
    })?;

    fut.await?;

    Ok(())
}
```

More details on the usage of this library can be found in the [API docs](https://awestlake87.github.io/pyo3-asyncio/master/doc) and the primer below.

#### PyO3 Native Rust Modules

PyO3 Asyncio can also be used to write native modules with async functions.

Add the `[lib]` section to `Cargo.toml` to make your library a `cdylib` that Python can import.

```toml
[lib]
name = "my_async_module"
crate-type = ["cdylib"]
```

Make your project depend on `pyo3` with the `extension-module` feature enabled and select your
`pyo3-asyncio` runtime:

For `async-std`:

```toml
[dependencies]
pyo3 = { version = "0.16", features = ["extension-module"] }
pyo3-asyncio = { version = "0.16", features = ["async-std-runtime"] }
async-std = "1.9"
```

For `tokio`:

```toml
[dependencies]
pyo3 = { version = "0.16", features = ["extension-module"] }
pyo3-asyncio = { version = "0.16", features = ["tokio-runtime"] }
tokio = "1.9"
```

Export an async function that makes use of `async-std`:

```rust
//! lib.rs

use pyo3::{prelude::*, wrap_pyfunction};

#[pyfunction]
fn rust_sleep(py: Python) -> PyResult<&PyAny> {
    pyo3_asyncio::async_std::future_into_py(py, async {
        async_std::task::sleep(std::time::Duration::from_secs(1)).await;
        Ok(())
    })
}

#[pymodule]
fn my_async_module(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rust_sleep, m)?)?;

    Ok(())
}

```

If you want to use `tokio` instead, here's what your module should look like:

```rust
//! lib.rs

use pyo3::{prelude::*, wrap_pyfunction};

#[pyfunction]
fn rust_sleep(py: Python) -> PyResult<&PyAny> {
    pyo3_asyncio::tokio::future_into_py(py, async {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Ok(())
    })
}

#[pymodule]
fn my_async_module(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rust_sleep, m)?)?;
    Ok(())
}
```

You can build your module with maturin (see the [Using Rust in Python](https://pyo3.rs/main/#using-rust-from-python) section in the PyO3 guide for setup instructions). After that you should be able to run the Python REPL to try it out.

```bash
maturin develop && python3
🔗 Found pyo3 bindings
🐍 Found CPython 3.8 at python3
    Finished dev [unoptimized + debuginfo] target(s) in 0.04s
Python 3.8.5 (default, Jan 27 2021, 15:41:15)
[GCC 9.3.0] on linux
Type "help", "copyright", "credits" or "license" for more information.
>>> import asyncio
>>>
>>> from my_async_module import rust_sleep
>>>
>>> async def main():
>>>     await rust_sleep()
>>>
>>> # should sleep for 1s
>>> asyncio.run(main())
>>>
```

### Awaiting an Async Python Function in Rust

Let's take a look at a dead simple async Python function:

```python
# Sleep for 1 second
async def py_sleep():
    await asyncio.sleep(1)
```

**Async functions in Python are simply functions that return a `coroutine` object**. For our purposes,
we really don't need to know much about these `coroutine` objects. The key factor here is that calling
an `async` function is _just like calling a regular function_, the only difference is that we have
to do something special with the object that it returns.

Normally in Python, that something special is the `await` keyword, but in order to await this
coroutine in Rust, we first need to convert it into Rust's version of a `coroutine`: a `Future`.
That's where `pyo3-asyncio` comes in.
[`pyo3_asyncio::into_future`](https://docs.rs/pyo3-asyncio/latest/pyo3_asyncio/fn.into_future.html)
performs this conversion for us:

```rust no_run
use pyo3::prelude::*;

#[pyo3_asyncio::tokio::main]
async fn main() -> PyResult<()> {
    let future = Python::with_gil(|py| -> PyResult<_> {
        // import the module containing the py_sleep function
        let example = py.import("example")?;

        // calling the py_sleep method like a normal function
        // returns a coroutine
        let coroutine = example.call_method0("py_sleep")?;

        // convert the coroutine into a Rust future using the
        // tokio runtime
        pyo3_asyncio::tokio::into_future(coroutine)
    })?;

    // await the future
    future.await?;

    Ok(())
}
```

> If you're interested in learning more about `coroutines` and `awaitables` in general, check out the
> [Python 3 `asyncio` docs](https://docs.python.org/3/library/asyncio-task.html) for more information.

### Awaiting a Rust Future in Python

Here we have the same async function as before written in Rust using the
[`async-std`](https://async.rs/) runtime:

```rust
/// Sleep for 1 second
async fn rust_sleep() {
    async_std::task::sleep(std::time::Duration::from_secs(1)).await;
}
```

Similar to Python, Rust's async functions also return a special object called a
`Future`:

```rust compile_fail
let future = rust_sleep();
```

We can convert this `Future` object into Python to make it `awaitable`. This tells Python that you
can use the `await` keyword with it. In order to do this, we'll call
[`pyo3_asyncio::async_std::future_into_py`](https://docs.rs/pyo3-asyncio/latest/pyo3_asyncio/async_std/fn.future_into_py.html):

```rust
use pyo3::prelude::*;

async fn rust_sleep() {
    async_std::task::sleep(std::time::Duration::from_secs(1)).await;
}

#[pyfunction]
fn call_rust_sleep(py: Python) -> PyResult<&PyAny> {
    pyo3_asyncio::async_std::future_into_py(py, async move {
        rust_sleep().await;
        Ok(())
    })
}
```

In Python, we can call this pyo3 function just like any other async function:

```python
from example import call_rust_sleep

async def rust_sleep():
    await call_rust_sleep()
```

## Managing Event Loops

Python's event loop requires some special treatment, especially regarding the main thread. Some of
Python's `asyncio` features, like proper signal handling, require control over the main thread, which
doesn't always play well with Rust.

Luckily, Rust's event loops are pretty flexible and don't _need_ control over the main thread, so in
`pyo3-asyncio`, we decided the best way to handle Rust/Python interop was to just surrender the main
thread to Python and run Rust's event loops in the background. Unfortunately, since most event loop
implementations _prefer_ control over the main thread, this can still make some things awkward.

### PyO3 Asyncio Initialization

Because Python needs to control the main thread, we can't use the convenient proc macros from Rust
runtimes to handle the `main` function or `#[test]` functions. Instead, the initialization for PyO3 has to be done from the `main` function and the main
thread must block on [`pyo3_asyncio::run_forever`](https://docs.rs/pyo3-asyncio/latest/pyo3_asyncio/fn.run_forever.html) or [`pyo3_asyncio::async_std::run_until_complete`](https://docs.rs/pyo3-asyncio/latest/pyo3_asyncio/async_std/fn.run_until_complete.html).

Because we have to block on one of those functions, we can't use [`#[async_std::main]`](https://docs.rs/async-std/latest/async_std/attr.main.html) or [`#[tokio::main]`](https://docs.rs/tokio/1.1.0/tokio/attr.main.html)
since it's not a good idea to make long blocking calls during an async function.

> Internally, these `#[main]` proc macros are expanded to something like this:
>
> ```rust compile_fail
> fn main() {
>     // your async main fn
>     async fn _main_impl() { /* ... */ }
>     Runtime::new().block_on(_main_impl());
> }
> ```
>
> Making a long blocking call inside the `Future` that's being driven by `block_on` prevents that
> thread from doing anything else and can spell trouble for some runtimes (also this will actually
> deadlock a single-threaded runtime!). Many runtimes have some sort of `spawn_blocking` mechanism
> that can avoid this problem, but again that's not something we can use here since we need it to
> block on the _main_ thread.

For this reason, `pyo3-asyncio` provides its own set of proc macros to provide you with this
initialization. These macros are intended to mirror the initialization of `async-std` and `tokio`
while also satisfying the Python runtime's needs.

Here's a full example of PyO3 initialization with the `async-std` runtime:

```rust no_run
use pyo3::prelude::*;

#[pyo3_asyncio::async_std::main]
async fn main() -> PyResult<()> {
    // PyO3 is initialized - Ready to go

    let fut = Python::with_gil(|py| -> PyResult<_> {
        let asyncio = py.import("asyncio")?;

        // convert asyncio.sleep into a Rust Future
        pyo3_asyncio::async_std::into_future(
            asyncio.call_method1("sleep", (1.into_py(py),))?
        )
    })?;

    fut.await?;

    Ok(())
}
```

#### A Note About `asyncio.run`

In Python 3.7+, the recommended way to run a top-level coroutine with `asyncio`
is with `asyncio.run`. In `v0.13` we recommended against using this function due to initialization issues, but in `v0.14` it's perfectly valid to use this function... with a caveat.

Since our Rust <--> Python conversions require a reference to the Python event loop, this poses a problem. Imagine we have a PyO3 Asyncio module that defines
a `rust_sleep` function like in previous examples. You might rightfully assume that you can call pass this directly into `asyncio.run` like this:

```python
import asyncio

from my_async_module import rust_sleep

asyncio.run(rust_sleep())
```

You might be surprised to find out that this throws an error:

```bash
Traceback (most recent call last):
  File "example.py", line 5, in <module>
    asyncio.run(rust_sleep())
RuntimeError: no running event loop
```

What's happening here is that we are calling `rust_sleep` _before_ the future is
actually running on the event loop created by `asyncio.run`. This is counter-intuitive, but expected behaviour, and unfortunately there doesn't seem to be a good way of solving this problem within PyO3 Asyncio itself.

However, we can make this example work with a simple workaround:

```python
import asyncio

from my_async_module import rust_sleep

# Calling main will just construct the coroutine that later calls rust_sleep.
# - This ensures that rust_sleep will be called when the event loop is running,
#   not before.
async def main():
    await rust_sleep()

# Run the main() coroutine at the top-level instead
asyncio.run(main())
```

#### Non-standard Python Event Loops

Python allows you to use alternatives to the default `asyncio` event loop. One
popular alternative is `uvloop`. In `v0.13` using non-standard event loops was
a bit of an ordeal, but in `v0.14` it's trivial.

#### Using `uvloop` in a PyO3 Asyncio Native Extensions

```toml
# Cargo.toml

[lib]
name = "my_async_module"
crate-type = ["cdylib"]

[dependencies]
pyo3 = { version = "0.16", features = ["extension-module"] }
pyo3-asyncio = { version = "0.16", features = ["tokio-runtime"] }
async-std = "1.9"
tokio = "1.9"
```

```rust
//! lib.rs

use pyo3::{prelude::*, wrap_pyfunction};

#[pyfunction]
fn rust_sleep(py: Python) -> PyResult<&PyAny> {
    pyo3_asyncio::tokio::future_into_py(py, async {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Ok(())
    })
}

#[pymodule]
fn my_async_module(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rust_sleep, m)?)?;

    Ok(())
}
```

```bash
$ maturin develop && python3
🔗 Found pyo3 bindings
🐍 Found CPython 3.8 at python3
    Finished dev [unoptimized + debuginfo] target(s) in 0.04s
Python 3.8.8 (default, Apr 13 2021, 19:58:26)
[GCC 7.3.0] :: Anaconda, Inc. on linux
Type "help", "copyright", "credits" or "license" for more information.
>>> import asyncio
>>> import uvloop
>>>
>>> import my_async_module
>>>
>>> uvloop.install()
>>>
>>> async def main():
...     await my_async_module.rust_sleep()
...
>>> asyncio.run(main())
>>>
```

#### Using `uvloop` in Rust Applications

Using `uvloop` in Rust applications is a bit trickier, but it's still possible
with relatively few modifications.

Unfortunately, we can't make use of the `#[pyo3_asyncio::<runtime>::main]` attribute with non-standard event loops. This is because the `#[pyo3_asyncio::<runtime>::main]` proc macro has to interact with the Python
event loop before we can install the `uvloop` policy.

```toml
[dependencies]
async-std = "1.9"
pyo3 = "0.16"
pyo3-asyncio = { version = "0.16", features = ["async-std-runtime"] }
```

```rust no_run
//! main.rs

use pyo3::{prelude::*, types::PyType};

fn main() -> PyResult<()> {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let uvloop = py.import("uvloop")?;
        uvloop.call_method0("install")?;

        // store a reference for the assertion
        let uvloop = PyObject::from(uvloop);

        pyo3_asyncio::async_std::run(py, async move {
            // verify that we are on a uvloop.Loop
            Python::with_gil(|py| -> PyResult<()> {
                assert!(uvloop
                    .as_ref(py)
                    .getattr("Loop")?
                    .downcast::<PyType>()
                    .unwrap()
                    .is_instance(pyo3_asyncio::async_std::get_current_loop(py)?)?);
                Ok(())
            })?;

            async_std::task::sleep(std::time::Duration::from_secs(1)).await;

            Ok(())
        })
    })
}
```

### Additional Information

- Managing event loop references can be tricky with pyo3-asyncio. See [Event Loop References and ContextVars](https://awestlake87.github.io/pyo3-asyncio/master/doc/pyo3_asyncio/#event-loop-references-and-contextvars) in the API docs to get a better intuition for how event loop references are managed in this library.
- Testing pyo3-asyncio libraries and applications requires a custom test harness since Python requires control over the main thread. You can find a testing guide in the [API docs for the `testing` module](https://awestlake87.github.io/pyo3-asyncio/master/doc/pyo3_asyncio/testing)

## Migration Guide

### Migrating from 0.13 to 0.14

So what's changed from `v0.13` to `v0.14`?

Well, a lot actually. There were some pretty major flaws in the initialization behaviour of `v0.13`. While it would have been nicer to address these issues without changing the public API, I decided it'd be better to break some of the old API rather than completely change the underlying behaviour of the existing functions. I realize this is going to be a bit of a headache, so hopefully this section will help you through it.

To make things a bit easier, I decided to keep most of the old API alongside the new one (with some deprecation warnings to encourage users to move away from it). It should be possible to use the `v0.13` API alongside the newer `v0.14` API, which should allow you to upgrade your application piecemeal rather than all at once.

**Before you get started, I personally recommend taking a look at [Event Loop References and ContextVars](https://awestlake87.github.io/pyo3-asyncio/master/doc/pyo3_asyncio/#event-loop-references-and-contextvars) in order to get a better grasp on the motivation behind these changes and the nuance involved in using the new conversions.**

### 0.14 Highlights

- Tokio initialization is now lazy.
  - No configuration necessary if you're using the multithreaded scheduler
  - Calls to `pyo3_asyncio::tokio::init_multithread` or `pyo3_asyncio::tokio::init_multithread_once` can just be removed.
  - Calls to `pyo3_asyncio::tokio::init_current_thread` or `pyo3_asyncio::tokio::init_current_thread_once` require some special attention.
  - Custom runtime configuration is done by passing a `tokio::runtime::Builder` into `pyo3_asyncio::tokio::init` instead of a `tokio::runtime::Runtime`
- A new, more correct set of functions has been added to replace the `v0.13` conversions.
  - `pyo3_asyncio::into_future_with_loop`
  - `pyo3_asyncio::<runtime>::future_into_py_with_loop`
  - `pyo3_asyncio::<runtime>::local_future_into_py_with_loop`
  - `pyo3_asyncio::<runtime>::into_future`
  - `pyo3_asyncio::<runtime>::future_into_py`
  - `pyo3_asyncio::<runtime>::local_future_into_py`
  - `pyo3_asyncio::<runtime>::get_current_loop`
- `pyo3_asyncio::try_init` is no longer required if you're only using `0.14` conversions
- The `ThreadPoolExecutor` is no longer configured automatically at the start.
  - Fortunately, this doesn't seem to have much effect on `v0.13` code, it just means that it's now possible to configure the executor manually as you see fit.

### Upgrading Your Code to 0.14

1. Fix PyO3 0.14 initialization.
   - PyO3 0.14 feature gated its automatic initialization behaviour behind "auto-initialize". You can either enable the "auto-initialize" behaviour in your project or add a call to `pyo3::prepare_freethreaded_python()` to the start of your program.
   - If you're using the `#[pyo3_asyncio::<runtime>::main]` proc macro attributes, then you can skip this step. `#[pyo3_asyncio::<runtime>::main]` will call `pyo3::prepare_freethreaded_python()` at the start regardless of your project's "auto-initialize" feature.
2. Fix the tokio initialization.

   - Calls to `pyo3_asyncio::tokio::init_multithread` or `pyo3_asyncio::tokio::init_multithread_once` can just be removed.
   - If you're using the current thread scheduler, you'll need to manually spawn the thread that it runs on during initialization:

     ```rust no_run
     let mut builder = tokio::runtime::Builder::new_current_thread();
     builder.enable_all();

     pyo3_asyncio::tokio::init(builder);
     std::thread::spawn(move || {
         pyo3_asyncio::tokio::get_runtime().block_on(
             futures::future::pending::<()>()
         );
     });
     ```

   - Custom `tokio::runtime::Builder` configs can be passed into `pyo3_asyncio::tokio::init`. The `tokio::runtime::Runtime` will be lazily instantiated on the first call to `pyo3_asyncio::tokio::get_runtime()`

3. If you're using `pyo3_asyncio::run_forever` in your application, you should switch to a more manual approach.

   > `run_forever` is not the recommended way of running an event loop in Python, so it might be a good idea to move away from it. This function would have needed to change for `0.14`, but since it's considered an edge case, it was decided that users could just manually call it if they need to.

   ```rust
   use pyo3::prelude::*;

   fn main() -> PyResult<()> {
       pyo3::prepare_freethreaded_python();

       Python::with_gil(|py| {
           let asyncio = py.import("asyncio")?;

           let event_loop = asyncio.call_method0("new_event_loop")?;
           asyncio.call_method1("set_event_loop", (event_loop,))?;

           let event_loop_hdl = PyObject::from(event_loop);

           pyo3_asyncio::tokio::get_runtime().spawn(async move {
               tokio::time::sleep(std::time::Duration::from_secs(1)).await;

               // Stop the event loop manually
               Python::with_gil(|py| {
                   event_loop_hdl
                       .as_ref(py)
                       .call_method1(
                           "call_soon_threadsafe",
                           (event_loop_hdl
                               .as_ref(py)
                               .getattr("stop")
                               .unwrap(),),
                       )
                       .unwrap();
               })
           });

           event_loop.call_method0("run_forever")?;
           Ok(())
       })
   }
   ```

4. Replace conversions with their newer counterparts.
   > You may encounter some issues regarding the usage of `get_running_loop` vs `get_event_loop`. For more details on these newer conversions and how they should be used see [Event Loop References and ContextVars](https://awestlake87.github.io/pyo3-asyncio/master/doc/pyo3_asyncio/#event-loop-references-and-contextvars).
   - Replace `pyo3_asyncio::into_future` with `pyo3_asyncio::<runtime>::into_future`
   - Replace `pyo3_asyncio::<runtime>::into_coroutine` with `pyo3_asyncio::<runtime>::future_into_py`
   - Replace `pyo3_asyncio::get_event_loop` with `pyo3_asyncio::<runtime>::get_current_loop`
5. After all conversions have been replaced with their `v0.14` counterparts, `pyo3_asyncio::try_init` can safely be removed.

> The `v0.13` API has been removed in version `v0.15`

### Migrating from 0.14 to 0.15+

There have been a few changes to the API in order to support proper cancellation from Python and the `contextvars` module.

- Any instance of `cancellable_future_into_py` and `local_cancellable_future_into_py` conversions can be replaced with their`future_into_py` and `local_future_into_py` counterparts.
  > Cancellation support became the default behaviour in 0.15.
- Instances of `*_with_loop` conversions should be replaced with the newer `*_with_locals` conversions.

  ```rust no_run
  use pyo3::prelude::*;

  Python::with_gil(|py| -> PyResult<()> {

      // *_with_loop conversions in 0.14
      //
      // let event_loop = pyo3_asyncio::get_running_loop(py)?;
      //
      // let fut = pyo3_asyncio::tokio::future_into_py_with_loop(
      //     event_loop,
      //     async move { Ok(Python::with_gil(|py| py.None())) }
      // )?;
      //
      // should be replaced with *_with_locals in 0.15+
      let fut = pyo3_asyncio::tokio::future_into_py_with_locals(
          py,
          pyo3_asyncio::tokio::get_current_locals(py)?,
          async move { Ok(()) }
      )?;

      Ok(())
  });
  ```

- `scope` and `scope_local` variants now accept `TaskLocals` instead of `event_loop`. You can usually just replace the `event_loop` with `pyo3_asyncio::TaskLocals::new(event_loop).copy_context(py)?`.
- Return types for `future_into_py`, `future_into_py_with_locals` `local_future_into_py`, and `local_future_into_py_with_locals` are now constrained by the bound `IntoPy<PyObject>` instead of requiring the return type to be `PyObject`. This can make the return types for futures more flexible, but inference can also fail when the concrete type is ambiguous (for example when using `into()`). Sometimes the `into()` can just be removed,
- `run`, and `run_until_complete` can now return any `Send + 'static` value.

### Migrating from 0.15 to 0.16

Actually, not much has changed in the API. I'm happy to say that the PyO3 Asyncio is reaching a
pretty stable point in 0.16. For the most part, 0.16 has been about cleanup and removing deprecated
functions from the API.

PyO3 0.16 comes with a few API changes of its own, but one of the changes that most impacted PyO3
Asyncio was it's decision to drop support for Python 3.6. PyO3 Asyncio has been using a few
workarounds / hacks to support the pre-3.7 version of Python's asyncio library that are no longer
necessary. PyO3 Asyncio's underlying implementation is now a bit cleaner because of this.

PyO3 Asyncio 0.15 included some important fixes to the API in order to add support for proper task
cancellation and allow for the preservation / use of contextvars in Python coroutines. This led to
the deprecation of some 0.14 functions that were used for edge cases in favor of some more correct
versions, and those deprecated functions are now removed from the API in 0.16.

In addition, with PyO3 Asyncio 0.16, the library now has experimental support for conversions from
Python's async generators into a Rust `Stream`. There are currently two versions `v1` and `v2` with
slightly different performance and type signatures, so I'm hoping to get some feedback on which one
works best for downstream users. Just enable the `unstable-streams` feature and you're good to go!

> The inverse conversion, Rust `Stream` to Python async generator, may come in a later release if
> requested!
