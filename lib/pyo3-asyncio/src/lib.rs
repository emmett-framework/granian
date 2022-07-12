#![warn(missing_docs)]

//! Rust Bindings to the Python Asyncio Event Loop
//!
//! # Motivation
//!
//! This crate aims to provide a convenient interface to manage the interop between Python and
//! Rust's async/await models. It supports conversions between Rust and Python futures and manages
//! the event loops for both languages. Python's threading model and GIL can make this interop a bit
//! trickier than one might expect, so there are a few caveats that users should be aware of.
//!
//! ## Why Two Event Loops
//!
//! Currently, we don't have a way to run Rust futures directly on Python's event loop. Likewise,
//! Python's coroutines cannot be directly spawned on a Rust event loop. The two coroutine models
//! require some additional assistance from their event loops, so in all likelihood they will need
//! a new _unique_ event loop that addresses the needs of both languages if the coroutines are to
//! be run on the same loop.
//!
//! It's not immediately clear that this would provide worthwhile performance wins either, so in the
//! interest of getting something simple out there to facilitate these conversions, this crate
//! handles the communication between _separate_ Python and Rust event loops.
//!
//! ## Python's Event Loop and the Main Thread
//!
//! Python is very picky about the threads used by the `asyncio` executor. In particular, it needs
//! to have control over the main thread in order to handle signals like CTRL-C correctly. This
//! means that Cargo's default test harness will no longer work since it doesn't provide a method of
//! overriding the main function to add our event loop initialization and finalization.
//!
//! ## Event Loop References and ContextVars
//!
//! One problem that arises when interacting with Python's asyncio library is that the functions we
//! use to get a reference to the Python event loop can only be called in certain contexts. Since
//! PyO3 Asyncio needs to interact with Python's event loop during conversions, the context of these
//! conversions can matter a lot.
//!
//! Likewise, Python's `contextvars` library can require some special treatment. Python functions
//! and coroutines can rely on the context of outer coroutines to function correctly, so this
//! library needs to be able to preserve `contextvars` during conversions.
//!
//! > The core conversions we've mentioned so far in the README should insulate you from these
//! concerns in most cases. For the edge cases where they don't, this section should provide you
//! with the information you need to solve these problems.
//!
//! ### The Main Dilemma
//!
//! Python programs can have many independent event loop instances throughout the lifetime of the
//! application (`asyncio.run` for example creates its own event loop each time it's called for
//! instance), and they can even run concurrent with other event loops. For this reason, the most
//! correct method of obtaining a reference to the Python event loop is via
//! `asyncio.get_running_loop`.
//!
//! `asyncio.get_running_loop` returns the event loop associated with the current OS thread. It can
//! be used inside Python coroutines to spawn concurrent tasks, interact with timers, or in our case
//! signal between Rust and Python. This is all well and good when we are operating on a Python
//! thread, but since Rust threads are not associated with a Python event loop,
//! `asyncio.get_running_loop` will fail when called on a Rust runtime.
//!
//! `contextvars` operates in a similar way, though the current context is not always associated
//! with the current OS thread. Different contexts can be associated with different coroutines even
//! if they run on the same OS thread.
//!
//! ### The Solution
//!
//! A really straightforward way of dealing with this problem is to pass references to the
//! associated Python event loop and context for every conversion. That's why we have a structure
//! called `TaskLocals` and a set of conversions that accept it.
//!
//! `TaskLocals` stores the current event loop, and allows the user to copy the current Python
//! context if necessary. The following conversions will use these references to perform the
//! necessary conversions and restore Python context when needed:
//!
//! - `pyo3_asyncio::into_future_with_locals` - Convert a Python awaitable into a Rust future.
//! - `pyo3_asyncio::<runtime>::future_into_py_with_locals` - Convert a Rust future into a Python
//! awaitable.
//! - `pyo3_asyncio::<runtime>::local_future_into_py_with_locals` - Convert a `!Send` Rust future
//! into a Python awaitable.
//!
//! One clear disadvantage to this approach is that the Rust application has to explicitly track
//! these references. In native libraries, we can't make any assumptions about the underlying event
//! loop, so the only reliable way to make sure our conversions work properly is to store these
//! references at the callsite to use later on.
//!
//! ```rust
//! use pyo3::{wrap_pyfunction, prelude::*};
//!
//! # #[cfg(feature = "tokio-runtime")]
//! #[pyfunction]
//! fn sleep(py: Python) -> PyResult<&PyAny> {
//!     // Construct the task locals structure with the current running loop and context
//!     let locals = pyo3_asyncio::TaskLocals::with_running_loop(py)?.copy_context(py)?;
//!
//!     // Convert the async move { } block to a Python awaitable
//!     pyo3_asyncio::tokio::future_into_py_with_locals(py, locals.clone(), async move {
//!         let py_sleep = Python::with_gil(|py| {
//!             // Sometimes we need to call other async Python functions within
//!             // this future. In order for this to work, we need to track the
//!             // event loop from earlier.
//!             pyo3_asyncio::into_future_with_locals(
//!                 &locals,
//!                 py.import("asyncio")?.call_method1("sleep", (1,))?
//!             )
//!         })?;
//!
//!         py_sleep.await?;
//!
//!         Ok(())
//!     })
//! }
//!
//! # #[cfg(feature = "tokio-runtime")]
//! #[pymodule]
//! fn my_mod(py: Python, m: &PyModule) -> PyResult<()> {
//!     m.add_function(wrap_pyfunction!(sleep, m)?)?;
//!     Ok(())
//! }
//! ```
//!
//! > A naive solution to this tracking problem would be to cache a global reference to the asyncio
//! event loop that all PyO3 Asyncio conversions can use. In fact this is what we did in PyO3
//! Asyncio `v0.13`. This works well for applications, but it soon became clear that this is not
//! so ideal for libraries. Libraries usually have no direct control over how the event loop is
//! managed, they're just expected to work with any event loop at any point in the application.
//! This problem is compounded further when multiple event loops are used in the application since
//! the global reference will only point to one.
//!
//! Another disadvantage to this explicit approach that is less obvious is that we can no longer
//! call our `#[pyfunction] fn sleep` on a Rust runtime since `asyncio.get_running_loop` only works
//! on Python threads! It's clear that we need a slightly more flexible approach.
//!
//! In order to detect the Python event loop at the callsite, we need something like
//! `asyncio.get_running_loop` and `contextvars.copy_context` that works for _both Python and Rust_.
//! In Python, `asyncio.get_running_loop` uses thread-local data to retrieve the event loop
//! associated with the current thread. What we need in Rust is something that can retrieve the
//! Python event loop and contextvars associated with the current Rust _task_.
//!
//! Enter `pyo3_asyncio::<runtime>::get_current_locals`. This function first checks task-local data
//! for the `TaskLocals`, then falls back on `asyncio.get_running_loop` and
//! `contextvars.copy_context` if no task locals are found. This way both bases are
//! covered.
//!
//! Now, all we need is a way to store the `TaskLocals` for the Rust future. Since this is a
//! runtime-specific feature, you can find the following functions in each runtime module:
//!
//! - `pyo3_asyncio::<runtime>::scope` - Store the task-local data when executing the given Future.
//! - `pyo3_asyncio::<runtime>::scope_local` - Store the task-local data when executing the given
//! `!Send` Future.
//!
//! With these new functions, we can make our previous example more correct:
//!
//! ```rust no_run
//! use pyo3::prelude::*;
//!
//! # #[cfg(feature = "tokio-runtime")]
//! #[pyfunction]
//! fn sleep(py: Python) -> PyResult<&PyAny> {
//!     // get the current event loop through task-local data
//!     // OR `asyncio.get_running_loop` and `contextvars.copy_context`
//!     let locals = pyo3_asyncio::tokio::get_current_locals(py)?;
//!
//!     pyo3_asyncio::tokio::future_into_py_with_locals(
//!         py,
//!         locals.clone(),
//!         // Store the current locals in task-local data
//!         pyo3_asyncio::tokio::scope(locals.clone(), async move {
//!             let py_sleep = Python::with_gil(|py| {
//!                 pyo3_asyncio::into_future_with_locals(
//!                     // Now we can get the current locals through task-local data
//!                     &pyo3_asyncio::tokio::get_current_locals(py)?,
//!                     py.import("asyncio")?.call_method1("sleep", (1,))?
//!                 )
//!             })?;
//!
//!             py_sleep.await?;
//!
//!             Ok(Python::with_gil(|py| py.None()))
//!         })
//!     )
//! }
//!
//! # #[cfg(feature = "tokio-runtime")]
//! #[pyfunction]
//! fn wrap_sleep(py: Python) -> PyResult<&PyAny> {
//!     // get the current event loop through task-local data
//!     // OR `asyncio.get_running_loop` and `contextvars.copy_context`
//!     let locals = pyo3_asyncio::tokio::get_current_locals(py)?;
//!
//!     pyo3_asyncio::tokio::future_into_py_with_locals(
//!         py,
//!         locals.clone(),
//!         // Store the current locals in task-local data
//!         pyo3_asyncio::tokio::scope(locals.clone(), async move {
//!             let py_sleep = Python::with_gil(|py| {
//!                 pyo3_asyncio::into_future_with_locals(
//!                     &pyo3_asyncio::tokio::get_current_locals(py)?,
//!                     // We can also call sleep within a Rust task since the
//!                     // locals are stored in task local data
//!                     sleep(py)?
//!                 )
//!             })?;
//!
//!             py_sleep.await?;
//!
//!             Ok(Python::with_gil(|py| py.None()))
//!         })
//!     )
//! }
//!
//! # #[cfg(feature = "tokio-runtime")]
//! #[pymodule]
//! fn my_mod(py: Python, m: &PyModule) -> PyResult<()> {
//!     m.add_function(wrap_pyfunction!(sleep, m)?)?;
//!     m.add_function(wrap_pyfunction!(wrap_sleep, m)?)?;
//!     Ok(())
//! }
//! ```
//!
//! Even though this is more correct, it's clearly not more ergonomic. That's why we introduced a
//! set of functions with this functionality baked in:
//!
//! - `pyo3_asyncio::<runtime>::into_future`
//!   > Convert a Python awaitable into a Rust future (using
//!   `pyo3_asyncio::<runtime>::get_current_locals`)
//! - `pyo3_asyncio::<runtime>::future_into_py`
//!   > Convert a Rust future into a Python awaitable (using
//!   `pyo3_asyncio::<runtime>::get_current_locals` and `pyo3_asyncio::<runtime>::scope` to set the
//!   task-local event loop for the given Rust future)
//! - `pyo3_asyncio::<runtime>::local_future_into_py`
//!   > Convert a `!Send` Rust future into a Python awaitable (using
//!   `pyo3_asyncio::<runtime>::get_current_locals` and `pyo3_asyncio::<runtime>::scope_local` to
//!   set the task-local event loop for the given Rust future).
//!
//! __These are the functions that we recommend using__. With these functions, the previous example
//! can be rewritten to be more compact:
//!
//! ```rust
//! use pyo3::prelude::*;
//!
//! # #[cfg(feature = "tokio-runtime")]
//! #[pyfunction]
//! fn sleep(py: Python) -> PyResult<&PyAny> {
//!     pyo3_asyncio::tokio::future_into_py(py, async move {
//!         let py_sleep = Python::with_gil(|py| {
//!             pyo3_asyncio::tokio::into_future(
//!                 py.import("asyncio")?.call_method1("sleep", (1,))?
//!             )
//!         })?;
//!
//!         py_sleep.await?;
//!
//!         Ok(Python::with_gil(|py| py.None()))
//!     })
//! }
//!
//! # #[cfg(feature = "tokio-runtime")]
//! #[pyfunction]
//! fn wrap_sleep(py: Python) -> PyResult<&PyAny> {
//!     pyo3_asyncio::tokio::future_into_py(py, async move {
//!         let py_sleep = Python::with_gil(|py| {
//!             pyo3_asyncio::tokio::into_future(sleep(py)?)
//!         })?;
//!
//!         py_sleep.await?;
//!
//!         Ok(Python::with_gil(|py| py.None()))
//!     })
//! }
//!
//! # #[cfg(feature = "tokio-runtime")]
//! #[pymodule]
//! fn my_mod(py: Python, m: &PyModule) -> PyResult<()> {
//!     m.add_function(wrap_pyfunction!(sleep, m)?)?;
//!     m.add_function(wrap_pyfunction!(wrap_sleep, m)?)?;
//!     Ok(())
//! }
//! ```
//!
//! > A special thanks to [@ShadowJonathan](https://github.com/ShadowJonathan) for helping with the
//! design and review of these changes!
//!
//! ## Rust's Event Loop
//!
//! Currently only the Async-Std and Tokio runtimes are supported by this crate. If you need support
//! for another runtime, feel free to make a request on GitHub (or attempt to add support yourself
//! with the [`generic`] module)!
//!
//! > _In the future, we may implement first class support for more Rust runtimes. Contributions are
//! welcome as well!_
//!
//! ## Features
//!
//! Items marked with
//! <span
//!   class="module-item stab portability"
//!   style="display: inline; border-radius: 3px; padding: 2px; font-size: 80%; line-height: 1.2;"
//! ><code>attributes</code></span>
//! are only available when the `attributes` Cargo feature is enabled:
//!
//! ```toml
//! [dependencies.pyo3-asyncio]
//! version = "0.16"
//! features = ["attributes"]
//! ```
//!
//! Items marked with
//! <span
//!   class="module-item stab portability"
//!   style="display: inline; border-radius: 3px; padding: 2px; font-size: 80%; line-height: 1.2;"
//! ><code>async-std-runtime</code></span>
//! are only available when the `async-std-runtime` Cargo feature is enabled:
//!
//! ```toml
//! [dependencies.pyo3-asyncio]
//! version = "0.16"
//! features = ["async-std-runtime"]
//! ```
//!
//! Items marked with
//! <span
//!   class="module-item stab portability"
//!   style="display: inline; border-radius: 3px; padding: 2px; font-size: 80%; line-height: 1.2;"
//! ><code>tokio-runtime</code></span>
//! are only available when the `tokio-runtime` Cargo feature is enabled:
//!
//! ```toml
//! [dependencies.pyo3-asyncio]
//! version = "0.16"
//! features = ["tokio-runtime"]
//! ```
//!
//! Items marked with
//! <span
//!   class="module-item stab portability"
//!   style="display: inline; border-radius: 3px; padding: 2px; font-size: 80%; line-height: 1.2;"
//! ><code>testing</code></span>
//! are only available when the `testing` Cargo feature is enabled:
//!
//! ```toml
//! [dependencies.pyo3-asyncio]
//! version = "0.16"
//! features = ["testing"]
//! ```

/// Re-exported for #[test] attributes
#[cfg(all(feature = "attributes", feature = "testing"))]
pub use inventory;

/// <span class="module-item stab portability" style="display: inline; border-radius: 3px; padding: 2px; font-size: 80%; line-height: 1.2;"><code>testing</code></span> Utilities for writing PyO3 Asyncio tests
#[cfg(feature = "testing")]
pub mod testing;

#[cfg(feature = "async-std")]
pub mod async_std;

#[cfg(feature = "tokio-runtime")]
pub mod tokio;

/// Errors and exceptions related to PyO3 Asyncio
pub mod err;

pub mod generic;

/// Test README
#[doc(hidden)]
pub mod doc_test {
    #[allow(unused)]
    macro_rules! doc_comment {
        ($x:expr, $module:item) => {
            #[doc = $x]
            $module
        };
    }

    #[allow(unused)]
    macro_rules! doctest {
        ($x:expr, $y:ident) => {
            doc_comment!(include_str!($x), mod $y {});
        };
    }

    #[cfg(all(
        feature = "async-std-runtime",
        feature = "tokio-runtime",
        feature = "attributes"
    ))]
    doctest!("../README.md", readme_md);
}

use std::future::Future;

use futures::channel::oneshot;
use once_cell::sync::OnceCell;
use pyo3::{
    prelude::*,
    types::{PyDict, PyTuple},
};

static ASYNCIO: OnceCell<PyObject> = OnceCell::new();
static CONTEXTVARS: OnceCell<PyObject> = OnceCell::new();
static ENSURE_FUTURE: OnceCell<PyObject> = OnceCell::new();
static GET_RUNNING_LOOP: OnceCell<PyObject> = OnceCell::new();

fn ensure_future<'p>(py: Python<'p>, awaitable: &'p PyAny) -> PyResult<&'p PyAny> {
    ENSURE_FUTURE
        .get_or_try_init(|| -> PyResult<PyObject> {
            Ok(asyncio(py)?.getattr("ensure_future")?.into())
        })?
        .as_ref(py)
        .call1((awaitable,))
}

pub fn create_future(event_loop: &PyAny) -> PyResult<&PyAny> {
    event_loop.call_method0("create_future")
}

fn close(event_loop: &PyAny) -> PyResult<()> {
    event_loop.call_method1(
        "run_until_complete",
        (event_loop.call_method0("shutdown_asyncgens")?,),
    )?;

    // how to do this prior to 3.9?
    if event_loop.hasattr("shutdown_default_executor")? {
        event_loop.call_method1(
            "run_until_complete",
            (event_loop.call_method0("shutdown_default_executor")?,),
        )?;
    }

    event_loop.call_method0("close")?;

    Ok(())
}

fn asyncio(py: Python) -> PyResult<&PyAny> {
    ASYNCIO
        .get_or_try_init(|| Ok(py.import("asyncio")?.into()))
        .map(|asyncio| asyncio.as_ref(py))
}

/// Get a reference to the Python Event Loop from Rust
///
/// Equivalent to `asyncio.get_running_loop()` in Python 3.7+.
pub fn get_running_loop(py: Python) -> PyResult<&PyAny> {
    // Ideally should call get_running_loop, but calls get_event_loop for compatibility when
    // get_running_loop is not available.
    GET_RUNNING_LOOP
        .get_or_try_init(|| -> PyResult<PyObject> {
            let asyncio = asyncio(py)?;

            Ok(asyncio.getattr("get_running_loop")?.into())
        })?
        .as_ref(py)
        .call0()
}

fn contextvars(py: Python) -> PyResult<&PyAny> {
    Ok(CONTEXTVARS
        .get_or_try_init(|| py.import("contextvars").map(|m| m.into()))?
        .as_ref(py))
}

fn copy_context(py: Python) -> PyResult<&PyAny> {
    contextvars(py)?.call_method0("copy_context")
}

/// Task-local data to store for Python conversions.
#[derive(Debug, Clone)]
pub struct TaskLocals {
    /// Track the event loop of the Python task
    event_loop: PyObject,
    /// Track the contextvars of the Python task
    context: PyObject,
}

impl TaskLocals {
    /// At a minimum, TaskLocals must store the event loop.
    pub fn new(event_loop: &PyAny) -> Self {
        Self {
            event_loop: event_loop.into(),
            context: event_loop.py().None(),
        }
    }

    /// Construct TaskLocals with the event loop returned by `get_running_loop`
    pub fn with_running_loop(py: Python) -> PyResult<Self> {
        Ok(Self::new(get_running_loop(py)?))
    }

    /// Manually provide the contextvars for the current task.
    pub fn with_context(self, context: &PyAny) -> Self {
        Self {
            context: context.into(),
            ..self
        }
    }

    /// Capture the current task's contextvars
    pub fn copy_context(self, py: Python) -> PyResult<Self> {
        Ok(self.with_context(copy_context(py)?))
    }

    /// Get a reference to the event loop
    pub fn event_loop<'p>(&self, py: Python<'p>) -> &'p PyAny {
        self.event_loop.clone().into_ref(py)
    }

    /// Get a reference to the python context
    pub fn context<'p>(&self, py: Python<'p>) -> &'p PyAny {
        self.context.clone().into_ref(py)
    }
}

#[pyclass]
struct PyTaskCompleter {
    tx: Option<oneshot::Sender<PyResult<PyObject>>>,
}

#[pymethods]
impl PyTaskCompleter {
    #[args(task)]
    pub fn __call__(&mut self, task: &PyAny) -> PyResult<()> {
        debug_assert!(task.call_method0("done")?.extract()?);

        let result = match task.call_method0("result") {
            Ok(val) => Ok(val.into()),
            Err(e) => Err(e),
        };

        // unclear to me whether or not this should be a panic or silent error.
        //
        // calling PyTaskCompleter twice should not be possible, but I don't think it really hurts
        // anything if it happens.
        if let Some(tx) = self.tx.take() {
            if tx.send(result).is_err() {
                // cancellation is not an error
            }
        }

        Ok(())
    }
}

#[pyclass]
struct PyEnsureFuture {
    awaitable: PyObject,
    tx: Option<oneshot::Sender<PyResult<PyObject>>>,
}

#[pymethods]
impl PyEnsureFuture {
    pub fn __call__(&mut self) -> PyResult<()> {
        Python::with_gil(|py| {
            let task = ensure_future(py, self.awaitable.as_ref(py))?;
            let on_complete = PyTaskCompleter { tx: self.tx.take() };
            task.call_method1("add_done_callback", (on_complete,))?;

            Ok(())
        })
    }
}

fn call_soon_threadsafe(
    event_loop: &PyAny,
    context: &PyAny,
    args: impl IntoPy<Py<PyTuple>>,
) -> PyResult<()> {
    let py = event_loop.py();

    let kwargs = PyDict::new(py);
    kwargs.set_item("context", context)?;

    event_loop.call_method("call_soon_threadsafe", args, Some(kwargs))?;
    Ok(())
}

/// Convert a Python `awaitable` into a Rust Future
///
/// This function converts the `awaitable` into a Python Task using `run_coroutine_threadsafe`. A
/// completion handler sends the result of this Task through a
/// `futures::channel::oneshot::Sender<PyResult<PyObject>>` and the future returned by this function
/// simply awaits the result through the `futures::channel::oneshot::Receiver<PyResult<PyObject>>`.
///
/// # Arguments
/// * `locals` - The Python event loop and context to be used for the provided awaitable
/// * `awaitable` - The Python `awaitable` to be converted
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use pyo3::prelude::*;
///
/// const PYTHON_CODE: &'static str = r#"
/// import asyncio
///
/// async def py_sleep(duration):
///     await asyncio.sleep(duration)
/// "#;
///
/// # #[cfg(feature = "tokio-runtime")]
/// async fn py_sleep(seconds: f32) -> PyResult<()> {
///     let test_mod = Python::with_gil(|py| -> PyResult<PyObject> {
///         Ok(
///             PyModule::from_code(
///                 py,
///                 PYTHON_CODE,
///                 "test_into_future/test_mod.py",
///                 "test_mod"
///             )?
///             .into()
///         )
///     })?;
///
///     Python::with_gil(|py| {
///         pyo3_asyncio::into_future_with_locals(
///             &pyo3_asyncio::tokio::get_current_locals(py)?,
///             test_mod
///                 .call_method1(py, "py_sleep", (seconds.into_py(py),))?
///                 .as_ref(py),
///         )
///     })?
///     .await?;
///     Ok(())
/// }
/// ```
pub fn into_future_with_locals(
    locals: &TaskLocals,
    awaitable: &PyAny,
) -> PyResult<impl Future<Output = PyResult<PyObject>> + Send> {
    let py = awaitable.py();
    let (tx, rx) = oneshot::channel();

    call_soon_threadsafe(
        locals.event_loop(py),
        locals.context(py),
        (PyEnsureFuture {
            awaitable: awaitable.into(),
            tx: Some(tx),
        },),
    )?;

    Ok(async move {
        match rx.await {
            Ok(item) => item,
            Err(_) => Python::with_gil(|py| {
                Err(PyErr::from_value(
                    asyncio(py)?.call_method0("CancelledError")?,
                ))
            }),
        }
    })
}

pub fn dump_err(py: Python<'_>) -> impl FnOnce(PyErr) + '_ {
    move |e| {
        // We can't display Python exceptions via std::fmt::Display,
        // so print the error here manually.
        e.print_and_set_sys_last_vars(py);
    }
}
