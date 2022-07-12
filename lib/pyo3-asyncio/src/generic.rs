//! Generic implementations of PyO3 Asyncio utilities that can be used for any Rust runtime
//!
//! Items marked with
//! <span
//!   class="module-item stab portability"
//!   style="display: inline; border-radius: 3px; padding: 2px; font-size: 80%; line-height: 1.2;"
//! ><code>unstable-streams</code></span>
//! are only available when the `unstable-streams` Cargo feature is enabled:
//!
//! ```toml
//! [dependencies.pyo3-asyncio]
//! version = "0.16"
//! features = ["unstable-streams"]
//! ```

use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use futures::{
    channel::{mpsc, oneshot},
    SinkExt,
};
use once_cell::sync::OnceCell;
use pin_project_lite::pin_project;
use pyo3::prelude::*;

#[allow(deprecated)]
use crate::{
    asyncio, call_soon_threadsafe, close, create_future, dump_err, err::RustPanic,
    get_running_loop, into_future_with_locals, TaskLocals,
};

/// Generic utilities for a JoinError
pub trait JoinError {
    /// Check if the spawned task exited because of a panic
    fn is_panic(&self) -> bool;
}

/// Generic Rust async/await runtime
pub trait Runtime: Send + 'static {
    /// The error returned by a JoinHandle after being awaited
    type JoinError: JoinError + Send;
    /// A future that completes with the result of the spawned task
    type JoinHandle: Future<Output = Result<(), Self::JoinError>> + Send;

    /// Spawn a future onto this runtime's event loop
    fn spawn<F>(fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + Send + 'static;
}

/// Extension trait for async/await runtimes that support spawning local tasks
pub trait SpawnLocalExt: Runtime {
    /// Spawn a !Send future onto this runtime's event loop
    fn spawn_local<F>(fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + 'static;
}

/// Exposes the utilities necessary for using task-local data in the Runtime
pub trait ContextExt: Runtime {
    /// Set the task locals for the given future
    fn scope<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
    where
        F: Future<Output = R> + Send + 'static;

    /// Get the task locals for the current task
    fn get_task_locals() -> Option<TaskLocals>;
}

/// Adds the ability to scope task-local data for !Send futures
pub trait LocalContextExt: Runtime {
    /// Set the task locals for the given !Send future
    fn scope_local<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R>>>
    where
        F: Future<Output = R> + 'static;
}

/// Get the current event loop from either Python or Rust async task local context
///
/// This function first checks if the runtime has a task-local reference to the Python event loop.
/// If not, it calls [`get_running_loop`](crate::get_running_loop`) to get the event loop associated
/// with the current OS thread.
pub fn get_current_loop<R>(py: Python) -> PyResult<&PyAny>
where
    R: ContextExt,
{
    if let Some(locals) = R::get_task_locals() {
        Ok(locals.event_loop.into_ref(py))
    } else {
        get_running_loop(py)
    }
}

/// Either copy the task locals from the current task OR get the current running loop and
/// contextvars from Python.
pub fn get_current_locals<R>(py: Python) -> PyResult<TaskLocals>
where
    R: ContextExt,
{
    if let Some(locals) = R::get_task_locals() {
        Ok(locals)
    } else {
        Ok(TaskLocals::with_running_loop(py)?.copy_context(py)?)
    }
}

/// Run the event loop until the given Future completes
///
/// After this function returns, the event loop can be resumed with [`run_until_complete`]
///
/// # Arguments
/// * `event_loop` - The Python event loop that should run the future
/// * `fut` - The future to drive to completion
///
/// # Examples
///
/// ```no_run
/// # use std::{task::{Context, Poll}, pin::Pin, future::Future};
/// #
/// # use pyo3_asyncio::{
/// #     TaskLocals,
/// #     generic::{JoinError, SpawnLocalExt, ContextExt, LocalContextExt, Runtime}
/// # };
/// #
/// # struct MyCustomJoinError;
/// #
/// # impl JoinError for MyCustomJoinError {
/// #     fn is_panic(&self) -> bool {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomJoinHandle;
/// #
/// # impl Future for MyCustomJoinHandle {
/// #     type Output = Result<(), MyCustomJoinError>;
/// #
/// #     fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomRuntime;
/// #
/// # impl Runtime for MyCustomRuntime {
/// #     type JoinError = MyCustomJoinError;
/// #     type JoinHandle = MyCustomJoinHandle;
/// #
/// #     fn spawn<F>(fut: F) -> Self::JoinHandle
/// #     where
/// #         F: Future<Output = ()> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl ContextExt for MyCustomRuntime {
/// #     fn scope<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
/// #     where
/// #         F: Future<Output = R> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// #     fn get_task_locals() -> Option<TaskLocals> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # use std::time::Duration;
/// #
/// # use pyo3::prelude::*;
/// #
/// # Python::with_gil(|py| -> PyResult<()> {
/// # let event_loop = py.import("asyncio")?.call_method0("new_event_loop")?;
/// # #[cfg(feature = "tokio-runtime")]
/// pyo3_asyncio::generic::run_until_complete::<MyCustomRuntime, _, _>(event_loop, async move {
///     tokio::time::sleep(Duration::from_secs(1)).await;
///     Ok(())
/// })?;
/// # Ok(())
/// # }).unwrap();
/// ```
pub fn run_until_complete<R, F, T>(event_loop: &PyAny, fut: F) -> PyResult<T>
where
    R: Runtime + ContextExt,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: Send + Sync + 'static,
{
    let py = event_loop.py();
    let result_tx = Arc::new(Mutex::new(None));
    let result_rx = Arc::clone(&result_tx);
    let coro = future_into_py_with_locals::<R, _, ()>(
        py,
        TaskLocals::new(event_loop).copy_context(py)?,
        async move {
            let val = fut.await?;
            if let Ok(mut result) = result_tx.lock() {
                *result = Some(val);
            }
            Ok(())
        },
    )?;

    event_loop.call_method1("run_until_complete", (coro,))?;

    let result = result_rx.lock().unwrap().take().unwrap();
    Ok(result)
}

/// Run the event loop until the given Future completes
///
/// # Arguments
/// * `py` - The current PyO3 GIL guard
/// * `fut` - The future to drive to completion
///
/// # Examples
///
/// ```no_run
/// # use std::{task::{Context, Poll}, pin::Pin, future::Future};
/// #
/// # use pyo3_asyncio::{
/// #     TaskLocals,
/// #     generic::{JoinError, SpawnLocalExt, ContextExt, LocalContextExt, Runtime}
/// # };
/// #
/// # struct MyCustomJoinError;
/// #
/// # impl JoinError for MyCustomJoinError {
/// #     fn is_panic(&self) -> bool {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomJoinHandle;
/// #
/// # impl Future for MyCustomJoinHandle {
/// #     type Output = Result<(), MyCustomJoinError>;
/// #
/// #     fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomRuntime;
/// #
/// # impl Runtime for MyCustomRuntime {
/// #     type JoinError = MyCustomJoinError;
/// #     type JoinHandle = MyCustomJoinHandle;
/// #
/// #     fn spawn<F>(fut: F) -> Self::JoinHandle
/// #     where
/// #         F: Future<Output = ()> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl ContextExt for MyCustomRuntime {
/// #     fn scope<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
/// #     where
/// #         F: Future<Output = R> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// #     fn get_task_locals() -> Option<TaskLocals> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # use std::time::Duration;
/// # async fn custom_sleep(_duration: Duration) { }
/// #
/// # use pyo3::prelude::*;
/// #
/// fn main() {
///     Python::with_gil(|py| {
///         pyo3_asyncio::generic::run::<MyCustomRuntime, _, _>(py, async move {
///             custom_sleep(Duration::from_secs(1)).await;
///             Ok(())
///         })
///         .map_err(|e| {
///             e.print_and_set_sys_last_vars(py);
///         })
///         .unwrap();
///     })
/// }
/// ```
pub fn run<R, F, T>(py: Python, fut: F) -> PyResult<T>
where
    R: Runtime + ContextExt,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: Send + Sync + 'static,
{
    let event_loop = asyncio(py)?.call_method0("new_event_loop")?;

    let result = run_until_complete::<R, F, T>(event_loop, fut);

    close(event_loop)?;

    result
}

pub fn cancelled(future: &PyAny) -> PyResult<bool> {
    future.getattr("cancelled")?.call0()?.is_true()
}

pub fn set_result(event_loop: &PyAny, future: &PyAny, result: PyResult<PyObject>) -> PyResult<()> {
    let py = event_loop.py();
    let none = py.None().into_ref(py);

    match result {
        Ok(val) => {
            let set_result = future.getattr("set_result")?;
            call_soon_threadsafe(event_loop, none, (set_result, val))?;
        }
        Err(err) => {
            let set_exception = future.getattr("set_exception")?;
            call_soon_threadsafe(event_loop, none, (set_exception, err))?;
        }
    }

    Ok(())
}

/// Convert a Python `awaitable` into a Rust Future
///
/// This function simply forwards the future and the task locals returned by [`get_current_locals`]
/// to [`into_future_with_locals`](`crate::into_future_with_locals`). See
/// [`into_future_with_locals`](`crate::into_future_with_locals`) for more details.
///
/// # Arguments
/// * `awaitable` - The Python `awaitable` to be converted
///
/// # Examples
///
/// ```no_run
/// # use std::{pin::Pin, future::Future, task::{Context, Poll}, time::Duration};
/// #
/// # use pyo3::prelude::*;
/// #
/// # use pyo3_asyncio::{
/// #     TaskLocals,
/// #     generic::{JoinError, SpawnLocalExt, ContextExt, LocalContextExt, Runtime}
/// # };
/// #
/// # struct MyCustomJoinError;
/// #
/// # impl JoinError for MyCustomJoinError {
/// #     fn is_panic(&self) -> bool {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomJoinHandle;
/// #
/// # impl Future for MyCustomJoinHandle {
/// #     type Output = Result<(), MyCustomJoinError>;
/// #
/// #     fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomRuntime;
/// #
/// # impl MyCustomRuntime {
/// #     async fn sleep(_: Duration) {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl Runtime for MyCustomRuntime {
/// #     type JoinError = MyCustomJoinError;
/// #     type JoinHandle = MyCustomJoinHandle;
/// #
/// #     fn spawn<F>(fut: F) -> Self::JoinHandle
/// #     where
/// #         F: Future<Output = ()> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl ContextExt for MyCustomRuntime {
/// #     fn scope<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
/// #     where
/// #         F: Future<Output = R> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// #     fn get_task_locals() -> Option<TaskLocals> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// const PYTHON_CODE: &'static str = r#"
/// import asyncio
///
/// async def py_sleep(duration):
///     await asyncio.sleep(duration)
/// "#;
///
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
///         pyo3_asyncio::generic::into_future::<MyCustomRuntime>(
///             test_mod
///                 .call_method1(py, "py_sleep", (seconds.into_py(py),))?
///                 .as_ref(py),
///         )
///     })?
///     .await?;
///     Ok(())
/// }
/// ```
pub fn into_future<R>(
    awaitable: &PyAny,
) -> PyResult<impl Future<Output = PyResult<PyObject>> + Send>
where
    R: Runtime + ContextExt,
{
    into_future_with_locals(&get_current_locals::<R>(awaitable.py())?, awaitable)
}

/// Convert a Rust Future into a Python awaitable with a generic runtime
///
/// If the `asyncio.Future` returned by this conversion is cancelled via `asyncio.Future.cancel`,
/// the Rust future will be cancelled as well (new behaviour in `v0.15`).
///
/// Python `contextvars` are preserved when calling async Python functions within the Rust future
/// via [`into_future`] (new behaviour in `v0.15`).
///
/// > Although `contextvars` are preserved for async Python functions, synchronous functions will
/// unfortunately fail to resolve them when called within the Rust future. This is because the
/// function is being called from a Rust thread, not inside an actual Python coroutine context.
/// >
/// > As a workaround, you can get the `contextvars` from the current task locals using
/// [`get_current_locals`] and [`TaskLocals::context`](`crate::TaskLocals::context`), then wrap your
/// synchronous function in a call to `contextvars.Context.run`. This will set the context, call the
/// synchronous function, and restore the previous context when it returns or raises an exception.
///
/// # Arguments
/// * `py` - PyO3 GIL guard
/// * `locals` - The task-local data for Python
/// * `fut` - The Rust future to be converted
///
/// # Examples
///
/// ```no_run
/// # use std::{task::{Context, Poll}, pin::Pin, future::Future};
/// #
/// # use pyo3_asyncio::{
/// #     TaskLocals,
/// #     generic::{JoinError, SpawnLocalExt, ContextExt, LocalContextExt, Runtime}
/// # };
/// #
/// # struct MyCustomJoinError;
/// #
/// # impl JoinError for MyCustomJoinError {
/// #     fn is_panic(&self) -> bool {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomJoinHandle;
/// #
/// # impl Future for MyCustomJoinHandle {
/// #     type Output = Result<(), MyCustomJoinError>;
/// #
/// #     fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomRuntime;
/// #
/// # impl MyCustomRuntime {
/// #     async fn sleep(_: Duration) {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl Runtime for MyCustomRuntime {
/// #     type JoinError = MyCustomJoinError;
/// #     type JoinHandle = MyCustomJoinHandle;
/// #
/// #     fn spawn<F>(fut: F) -> Self::JoinHandle
/// #     where
/// #         F: Future<Output = ()> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl ContextExt for MyCustomRuntime {
/// #     fn scope<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
/// #     where
/// #         F: Future<Output = R> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// #     fn get_task_locals() -> Option<TaskLocals> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// use std::time::Duration;
///
/// use pyo3::prelude::*;
///
/// /// Awaitable sleep function
/// #[pyfunction]
/// fn sleep_for<'p>(py: Python<'p>, secs: &'p PyAny) -> PyResult<&'p PyAny> {
///     let secs = secs.extract()?;
///     pyo3_asyncio::generic::future_into_py_with_locals::<MyCustomRuntime, _, _>(
///         py,
///         pyo3_asyncio::generic::get_current_locals::<MyCustomRuntime>(py)?,
///         async move {
///             MyCustomRuntime::sleep(Duration::from_secs(secs)).await;
///             Ok(())
///         }
///     )
/// }
/// ```
pub fn future_into_py_with_locals<R, F, T>(
    py: Python,
    locals: TaskLocals,
    fut: F,
) -> PyResult<&PyAny>
where
    R: Runtime + ContextExt,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPy<PyObject>,
{
    let (cancel_tx, cancel_rx) = oneshot::channel();

    let py_fut = create_future(locals.event_loop.clone().into_ref(py))?;
    py_fut.call_method1(
        "add_done_callback",
        (PyDoneCallback {
            cancel_tx: Some(cancel_tx),
        },),
    )?;

    let future_tx1 = PyObject::from(py_fut);
    let future_tx2 = future_tx1.clone();

    R::spawn(async move {
        let locals2 = locals.clone();

        if let Err(e) = R::spawn(async move {
            let result = R::scope(
                locals2.clone(),
                Cancellable::new_with_cancel_rx(fut, cancel_rx),
            )
            .await;

            Python::with_gil(move |py| {
                if cancelled(future_tx1.as_ref(py))
                    .map_err(dump_err(py))
                    .unwrap_or(false)
                {
                    return;
                }

                let _ = set_result(
                    locals2.event_loop(py),
                    future_tx1.as_ref(py),
                    result.map(|val| val.into_py(py)),
                )
                .map_err(dump_err(py));
            });
        })
        .await
        {
            if e.is_panic() {
                Python::with_gil(move |py| {
                    if cancelled(future_tx2.as_ref(py))
                        .map_err(dump_err(py))
                        .unwrap_or(false)
                    {
                        return;
                    }

                    let _ = set_result(
                        locals.event_loop.as_ref(py),
                        future_tx2.as_ref(py),
                        Err(RustPanic::new_err("rust future panicked")),
                    )
                    .map_err(dump_err(py));
                });
            }
        }
    });

    Ok(py_fut)
}

pin_project! {
    /// Future returned by [`timeout`](timeout) and [`timeout_at`](timeout_at).
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    #[derive(Debug)]
    pub struct Cancellable<T> {
        #[pin]
        future: T,
        #[pin]
        cancel_rx: oneshot::Receiver<()>,

        poll_cancel_rx: bool
    }
}

impl<T> Cancellable<T> {
    pub fn new_with_cancel_rx(future: T, cancel_rx: oneshot::Receiver<()>) -> Self {
        Self {
            future,
            cancel_rx,

            poll_cancel_rx: true,
        }
    }
}

impl<F, T> Future for Cancellable<F>
where
    F: Future<Output = PyResult<T>>,
    T: IntoPy<PyObject>,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        // First, try polling the future
        if let Poll::Ready(v) = this.future.poll(cx) {
            return Poll::Ready(v);
        }

        // Now check for cancellation
        if *this.poll_cancel_rx {
            match this.cancel_rx.poll(cx) {
                Poll::Ready(Ok(())) => {
                    *this.poll_cancel_rx = false;
                    // The python future has already been cancelled, so this return value will never
                    // be used.
                    Poll::Ready(Err(pyo3::exceptions::PyBaseException::new_err(
                        "unreachable",
                    )))
                }
                Poll::Ready(Err(_)) => {
                    *this.poll_cancel_rx = false;
                    Poll::Pending
                }
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Pending
        }
    }
}

#[pyclass]
pub struct PyDoneCallback {
    pub cancel_tx: Option<oneshot::Sender<()>>,
}

#[pymethods]
impl PyDoneCallback {
    pub fn __call__(&mut self, fut: &PyAny) -> PyResult<()> {
        let py = fut.py();

        if cancelled(fut).map_err(dump_err(py)).unwrap_or(false) {
            let _ = self.cancel_tx.take().unwrap().send(());
        }

        Ok(())
    }
}

/// Convert a Rust Future into a Python awaitable with a generic runtime
///
/// If the `asyncio.Future` returned by this conversion is cancelled via `asyncio.Future.cancel`,
/// the Rust future will be cancelled as well (new behaviour in `v0.15`).
///
/// Python `contextvars` are preserved when calling async Python functions within the Rust future
/// via [`into_future`] (new behaviour in `v0.15`).
///
/// > Although `contextvars` are preserved for async Python functions, synchronous functions will
/// unfortunately fail to resolve them when called within the Rust future. This is because the
/// function is being called from a Rust thread, not inside an actual Python coroutine context.
/// >
/// > As a workaround, you can get the `contextvars` from the current task locals using
/// [`get_current_locals`] and [`TaskLocals::context`](`crate::TaskLocals::context`), then wrap your
/// synchronous function in a call to `contextvars.Context.run`. This will set the context, call the
/// synchronous function, and restore the previous context when it returns or raises an exception.
///
/// # Arguments
/// * `py` - The current PyO3 GIL guard
/// * `fut` - The Rust future to be converted
///
/// # Examples
///
/// ```no_run
/// # use std::{task::{Context, Poll}, pin::Pin, future::Future};
/// #
/// # use pyo3_asyncio::{
/// #     TaskLocals,
/// #     generic::{JoinError, SpawnLocalExt, ContextExt, LocalContextExt, Runtime}
/// # };
/// #
/// # struct MyCustomJoinError;
/// #
/// # impl JoinError for MyCustomJoinError {
/// #     fn is_panic(&self) -> bool {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomJoinHandle;
/// #
/// # impl Future for MyCustomJoinHandle {
/// #     type Output = Result<(), MyCustomJoinError>;
/// #
/// #     fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomRuntime;
/// #
/// # impl MyCustomRuntime {
/// #     async fn sleep(_: Duration) {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl Runtime for MyCustomRuntime {
/// #     type JoinError = MyCustomJoinError;
/// #     type JoinHandle = MyCustomJoinHandle;
/// #
/// #     fn spawn<F>(fut: F) -> Self::JoinHandle
/// #     where
/// #         F: Future<Output = ()> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl ContextExt for MyCustomRuntime {
/// #     fn scope<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
/// #     where
/// #         F: Future<Output = R> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// #     fn get_task_locals() -> Option<TaskLocals> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// use std::time::Duration;
///
/// use pyo3::prelude::*;
///
/// /// Awaitable sleep function
/// #[pyfunction]
/// fn sleep_for<'p>(py: Python<'p>, secs: &'p PyAny) -> PyResult<&'p PyAny> {
///     let secs = secs.extract()?;
///     pyo3_asyncio::generic::future_into_py::<MyCustomRuntime, _, _>(py, async move {
///         MyCustomRuntime::sleep(Duration::from_secs(secs)).await;
///         Ok(())
///     })
/// }
/// ```
pub fn future_into_py<R, F, T>(py: Python, fut: F) -> PyResult<&PyAny>
where
    R: Runtime + ContextExt,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPy<PyObject>,
{
    future_into_py_with_locals::<R, F, T>(py, get_current_locals::<R>(py)?, fut)
}

/// Convert a `!Send` Rust Future into a Python awaitable with a generic runtime and manual
/// specification of task locals.
///
/// If the `asyncio.Future` returned by this conversion is cancelled via `asyncio.Future.cancel`,
/// the Rust future will be cancelled as well (new behaviour in `v0.15`).
///
/// Python `contextvars` are preserved when calling async Python functions within the Rust future
/// via [`into_future`] (new behaviour in `v0.15`).
///
/// > Although `contextvars` are preserved for async Python functions, synchronous functions will
/// unfortunately fail to resolve them when called within the Rust future. This is because the
/// function is being called from a Rust thread, not inside an actual Python coroutine context.
/// >
/// > As a workaround, you can get the `contextvars` from the current task locals using
/// [`get_current_locals`] and [`TaskLocals::context`](`crate::TaskLocals::context`), then wrap your
/// synchronous function in a call to `contextvars.Context.run`. This will set the context, call the
/// synchronous function, and restore the previous context when it returns or raises an exception.
///
/// # Arguments
/// * `py` - PyO3 GIL guard
/// * `locals` - The task locals for the future
/// * `fut` - The Rust future to be converted
///
/// # Examples
///
/// ```no_run
/// # use std::{task::{Context, Poll}, pin::Pin, future::Future};
/// #
/// # use pyo3_asyncio::{
/// #     TaskLocals,
/// #     generic::{JoinError, SpawnLocalExt, ContextExt, LocalContextExt, Runtime}
/// # };
/// #
/// # struct MyCustomJoinError;
/// #
/// # impl JoinError for MyCustomJoinError {
/// #     fn is_panic(&self) -> bool {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomJoinHandle;
/// #
/// # impl Future for MyCustomJoinHandle {
/// #     type Output = Result<(), MyCustomJoinError>;
/// #
/// #     fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomRuntime;
/// #
/// # impl MyCustomRuntime {
/// #     async fn sleep(_: Duration) {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl Runtime for MyCustomRuntime {
/// #     type JoinError = MyCustomJoinError;
/// #     type JoinHandle = MyCustomJoinHandle;
/// #
/// #     fn spawn<F>(fut: F) -> Self::JoinHandle
/// #     where
/// #         F: Future<Output = ()> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl ContextExt for MyCustomRuntime {
/// #     fn scope<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
/// #     where
/// #         F: Future<Output = R> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// #     fn get_task_locals() -> Option<TaskLocals> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl SpawnLocalExt for MyCustomRuntime {
/// #     fn spawn_local<F>(fut: F) -> Self::JoinHandle
/// #     where
/// #         F: Future<Output = ()> + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl LocalContextExt for MyCustomRuntime {
/// #     fn scope_local<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R>>>
/// #     where
/// #         F: Future<Output = R> + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// use std::{rc::Rc, time::Duration};
///
/// use pyo3::prelude::*;
///
/// /// Awaitable sleep function
/// #[pyfunction]
/// fn sleep_for(py: Python, secs: u64) -> PyResult<&PyAny> {
///     // Rc is !Send so it cannot be passed into pyo3_asyncio::generic::future_into_py
///     let secs = Rc::new(secs);
///
///     pyo3_asyncio::generic::local_future_into_py_with_locals::<MyCustomRuntime, _, _>(
///         py,
///         pyo3_asyncio::generic::get_current_locals::<MyCustomRuntime>(py)?,
///         async move {
///             MyCustomRuntime::sleep(Duration::from_secs(*secs)).await;
///             Ok(())
///         }
///     )
/// }
/// ```
pub fn local_future_into_py_with_locals<R, F, T>(
    py: Python,
    locals: TaskLocals,
    fut: F,
) -> PyResult<&PyAny>
where
    R: Runtime + SpawnLocalExt + LocalContextExt,
    F: Future<Output = PyResult<T>> + 'static,
    T: IntoPy<PyObject>,
{
    let (cancel_tx, cancel_rx) = oneshot::channel();

    let py_fut = create_future(locals.event_loop.clone().into_ref(py))?;
    py_fut.call_method1(
        "add_done_callback",
        (PyDoneCallback {
            cancel_tx: Some(cancel_tx),
        },),
    )?;

    let future_tx1 = PyObject::from(py_fut);
    let future_tx2 = future_tx1.clone();

    R::spawn_local(async move {
        let locals2 = locals.clone();

        if let Err(e) = R::spawn_local(async move {
            let result = R::scope_local(
                locals2.clone(),
                Cancellable::new_with_cancel_rx(fut, cancel_rx),
            )
            .await;

            Python::with_gil(move |py| {
                if cancelled(future_tx1.as_ref(py))
                    .map_err(dump_err(py))
                    .unwrap_or(false)
                {
                    return;
                }

                let _ = set_result(
                    locals2.event_loop.as_ref(py),
                    future_tx1.as_ref(py),
                    result.map(|val| val.into_py(py)),
                )
                .map_err(dump_err(py));
            });
        })
        .await
        {
            if e.is_panic() {
                Python::with_gil(move |py| {
                    if cancelled(future_tx2.as_ref(py))
                        .map_err(dump_err(py))
                        .unwrap_or(false)
                    {
                        return;
                    }

                    let _ = set_result(
                        locals.event_loop.as_ref(py),
                        future_tx2.as_ref(py),
                        Err(RustPanic::new_err("Rust future panicked")),
                    )
                    .map_err(dump_err(py));
                });
            }
        }
    });

    Ok(py_fut)
}

/// Convert a `!Send` Rust Future into a Python awaitable with a generic runtime
///
/// If the `asyncio.Future` returned by this conversion is cancelled via `asyncio.Future.cancel`,
/// the Rust future will be cancelled as well (new behaviour in `v0.15`).
///
/// Python `contextvars` are preserved when calling async Python functions within the Rust future
/// via [`into_future`] (new behaviour in `v0.15`).
///
/// > Although `contextvars` are preserved for async Python functions, synchronous functions will
/// unfortunately fail to resolve them when called within the Rust future. This is because the
/// function is being called from a Rust thread, not inside an actual Python coroutine context.
/// >
/// > As a workaround, you can get the `contextvars` from the current task locals using
/// [`get_current_locals`] and [`TaskLocals::context`](`crate::TaskLocals::context`), then wrap your
/// synchronous function in a call to `contextvars.Context.run`. This will set the context, call the
/// synchronous function, and restore the previous context when it returns or raises an exception.
///
/// # Arguments
/// * `py` - The current PyO3 GIL guard
/// * `fut` - The Rust future to be converted
///
/// # Examples
///
/// ```no_run
/// # use std::{task::{Context, Poll}, pin::Pin, future::Future};
/// #
/// # use pyo3_asyncio::{
/// #     TaskLocals,
/// #     generic::{JoinError, SpawnLocalExt, ContextExt, LocalContextExt, Runtime}
/// # };
/// #
/// # struct MyCustomJoinError;
/// #
/// # impl JoinError for MyCustomJoinError {
/// #     fn is_panic(&self) -> bool {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomJoinHandle;
/// #
/// # impl Future for MyCustomJoinHandle {
/// #     type Output = Result<(), MyCustomJoinError>;
/// #
/// #     fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomRuntime;
/// #
/// # impl MyCustomRuntime {
/// #     async fn sleep(_: Duration) {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl Runtime for MyCustomRuntime {
/// #     type JoinError = MyCustomJoinError;
/// #     type JoinHandle = MyCustomJoinHandle;
/// #
/// #     fn spawn<F>(fut: F) -> Self::JoinHandle
/// #     where
/// #         F: Future<Output = ()> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl ContextExt for MyCustomRuntime {
/// #     fn scope<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
/// #     where
/// #         F: Future<Output = R> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// #     fn get_task_locals() -> Option<TaskLocals> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl SpawnLocalExt for MyCustomRuntime {
/// #     fn spawn_local<F>(fut: F) -> Self::JoinHandle
/// #     where
/// #         F: Future<Output = ()> + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl LocalContextExt for MyCustomRuntime {
/// #     fn scope_local<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R>>>
/// #     where
/// #         F: Future<Output = R> + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// use std::{rc::Rc, time::Duration};
///
/// use pyo3::prelude::*;
///
/// /// Awaitable sleep function
/// #[pyfunction]
/// fn sleep_for(py: Python, secs: u64) -> PyResult<&PyAny> {
///     // Rc is !Send so it cannot be passed into pyo3_asyncio::generic::future_into_py
///     let secs = Rc::new(secs);
///
///     pyo3_asyncio::generic::local_future_into_py::<MyCustomRuntime, _, _>(py, async move {
///         MyCustomRuntime::sleep(Duration::from_secs(*secs)).await;
///         Ok(())
///     })
/// }
/// ```
pub fn local_future_into_py<R, F, T>(py: Python, fut: F) -> PyResult<&PyAny>
where
    R: Runtime + ContextExt + SpawnLocalExt + LocalContextExt,
    F: Future<Output = PyResult<T>> + 'static,
    T: IntoPy<PyObject>,
{
    local_future_into_py_with_locals::<R, F, T>(py, get_current_locals::<R>(py)?, fut)
}

/// <span class="module-item stab portability" style="display: inline; border-radius: 3px; padding: 2px; font-size: 80%; line-height: 1.2;"><code>unstable-streams</code></span> Convert an async generator into a stream
///
/// **This API is marked as unstable** and is only available when the
/// `unstable-streams` crate feature is enabled. This comes with no
/// stability guarantees, and could be changed or removed at any time.
///
/// # Arguments
/// * `locals` - The current task locals
/// * `gen` - The Python async generator to be converted
///
/// # Examples
/// ```no_run
/// # use std::{task::{Context, Poll}, pin::Pin, future::Future};
/// #
/// # use pyo3_asyncio::{
/// #     TaskLocals,
/// #     generic::{JoinError, ContextExt, Runtime}
/// # };
/// #
/// # struct MyCustomJoinError;
/// #
/// # impl JoinError for MyCustomJoinError {
/// #     fn is_panic(&self) -> bool {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomJoinHandle;
/// #
/// # impl Future for MyCustomJoinHandle {
/// #     type Output = Result<(), MyCustomJoinError>;
/// #
/// #     fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomRuntime;
/// #
/// # impl Runtime for MyCustomRuntime {
/// #     type JoinError = MyCustomJoinError;
/// #     type JoinHandle = MyCustomJoinHandle;
/// #
/// #     fn spawn<F>(fut: F) -> Self::JoinHandle
/// #     where
/// #         F: Future<Output = ()> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl ContextExt for MyCustomRuntime {
/// #     fn scope<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
/// #     where
/// #         F: Future<Output = R> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// #     fn get_task_locals() -> Option<TaskLocals> {
/// #         unreachable!()
/// #     }
/// # }
///
/// use pyo3::prelude::*;
/// use futures::{StreamExt, TryStreamExt};
///
/// const TEST_MOD: &str = r#"
/// import asyncio
///
/// async def gen():
///     for i in range(10):
///         await asyncio.sleep(0.1)
///         yield i
/// "#;
///
/// # async fn test_async_gen() -> PyResult<()> {
/// let stream = Python::with_gil(|py| {
///     let test_mod = PyModule::from_code(
///         py,
///         TEST_MOD,
///         "test_rust_coroutine/test_mod.py",
///         "test_mod",
///     )?;
///
///     pyo3_asyncio::generic::into_stream_with_locals_v1::<MyCustomRuntime>(
///         pyo3_asyncio::generic::get_current_locals::<MyCustomRuntime>(py)?,
///         test_mod.call_method0("gen")?
///     )
/// })?;
///
/// let vals = stream
///     .map(|item| Python::with_gil(|py| -> PyResult<i32> { Ok(item?.as_ref(py).extract()?) }))
///     .try_collect::<Vec<i32>>()
///     .await?;
///
/// assert_eq!((0..10).collect::<Vec<i32>>(), vals);
///
/// Ok(())
/// # }
/// ```
#[cfg(feature = "unstable-streams")]
pub fn into_stream_with_locals_v1<'p, R>(
    locals: TaskLocals,
    gen: &'p PyAny,
) -> PyResult<impl futures::Stream<Item = PyResult<PyObject>> + 'static>
where
    R: Runtime,
{
    let (tx, rx) = async_channel::bounded(1);
    let anext = PyObject::from(gen.getattr("__anext__")?);

    R::spawn(async move {
        loop {
            let fut = Python::with_gil(|py| -> PyResult<_> {
                into_future_with_locals(&locals, anext.as_ref(py).call0()?)
            });
            let item = match fut {
                Ok(fut) => match fut.await {
                    Ok(item) => Ok(item),
                    Err(e) => {
                        let stop_iter = Python::with_gil(|py| {
                            e.is_instance_of::<pyo3::exceptions::PyStopAsyncIteration>(py)
                        });

                        if stop_iter {
                            // end the iteration
                            break;
                        } else {
                            Err(e)
                        }
                    }
                },
                Err(e) => Err(e),
            };

            if tx.send(item).await.is_err() {
                // receiving side was dropped
                break;
            }
        }
    });

    Ok(rx)
}

/// <span class="module-item stab portability" style="display: inline; border-radius: 3px; padding: 2px; font-size: 80%; line-height: 1.2;"><code>unstable-streams</code></span> Convert an async generator into a stream
///
/// **This API is marked as unstable** and is only available when the
/// `unstable-streams` crate feature is enabled. This comes with no
/// stability guarantees, and could be changed or removed at any time.
///
/// # Arguments
/// * `gen` - The Python async generator to be converted
///
/// # Examples
/// ```no_run
/// # use std::{task::{Context, Poll}, pin::Pin, future::Future};
/// #
/// # use pyo3_asyncio::{
/// #     TaskLocals,
/// #     generic::{JoinError, ContextExt, Runtime}
/// # };
/// #
/// # struct MyCustomJoinError;
/// #
/// # impl JoinError for MyCustomJoinError {
/// #     fn is_panic(&self) -> bool {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomJoinHandle;
/// #
/// # impl Future for MyCustomJoinHandle {
/// #     type Output = Result<(), MyCustomJoinError>;
/// #
/// #     fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomRuntime;
/// #
/// # impl Runtime for MyCustomRuntime {
/// #     type JoinError = MyCustomJoinError;
/// #     type JoinHandle = MyCustomJoinHandle;
/// #
/// #     fn spawn<F>(fut: F) -> Self::JoinHandle
/// #     where
/// #         F: Future<Output = ()> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl ContextExt for MyCustomRuntime {
/// #     fn scope<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
/// #     where
/// #         F: Future<Output = R> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// #     fn get_task_locals() -> Option<TaskLocals> {
/// #         unreachable!()
/// #     }
/// # }
///
/// use pyo3::prelude::*;
/// use futures::{StreamExt, TryStreamExt};
///
/// const TEST_MOD: &str = r#"
/// import asyncio
///
/// async def gen():
///     for i in range(10):
///         await asyncio.sleep(0.1)
///         yield i
/// "#;
///
/// # async fn test_async_gen() -> PyResult<()> {
/// let stream = Python::with_gil(|py| {
///     let test_mod = PyModule::from_code(
///         py,
///         TEST_MOD,
///         "test_rust_coroutine/test_mod.py",
///         "test_mod",
///     )?;
///
///     pyo3_asyncio::generic::into_stream_v1::<MyCustomRuntime>(test_mod.call_method0("gen")?)
/// })?;
///
/// let vals = stream
///     .map(|item| Python::with_gil(|py| -> PyResult<i32> { Ok(item?.as_ref(py).extract()?) }))
///     .try_collect::<Vec<i32>>()
///     .await?;
///
/// assert_eq!((0..10).collect::<Vec<i32>>(), vals);
///
/// Ok(())
/// # }
/// ```
#[cfg(feature = "unstable-streams")]
pub fn into_stream_v1<'p, R>(
    gen: &'p PyAny,
) -> PyResult<impl futures::Stream<Item = PyResult<PyObject>> + 'static>
where
    R: Runtime + ContextExt,
{
    into_stream_with_locals_v1::<R>(get_current_locals::<R>(gen.py())?, gen)
}

fn py_true() -> PyObject {
    static TRUE: OnceCell<PyObject> = OnceCell::new();
    TRUE.get_or_init(|| Python::with_gil(|py| true.into_py(py)))
        .clone()
}
fn py_false() -> PyObject {
    static FALSE: OnceCell<PyObject> = OnceCell::new();
    FALSE
        .get_or_init(|| Python::with_gil(|py| false.into_py(py)))
        .clone()
}

trait Sender: Send + 'static {
    fn send(&mut self, locals: TaskLocals, item: PyObject) -> PyResult<PyObject>;
    fn close(&mut self) -> PyResult<()>;
}

struct GenericSender<R>
where
    R: Runtime,
{
    runtime: PhantomData<R>,
    tx: mpsc::Sender<PyObject>,
}

impl<R> Sender for GenericSender<R>
where
    R: Runtime + ContextExt,
{
    fn send(&mut self, locals: TaskLocals, item: PyObject) -> PyResult<PyObject> {
        match self.tx.try_send(item.clone()) {
            Ok(_) => Ok(py_true()),
            Err(e) => {
                if e.is_full() {
                    let mut tx = self.tx.clone();
                    Python::with_gil(move |py| {
                        Ok(
                            future_into_py_with_locals::<R, _, PyObject>(py, locals, async move {
                                if tx.flush().await.is_err() {
                                    // receiving side disconnected
                                    return Ok(py_false());
                                }
                                if tx.send(item).await.is_err() {
                                    // receiving side disconnected
                                    return Ok(py_false());
                                }
                                Ok(py_true())
                            })?
                            .into(),
                        )
                    })
                } else {
                    Ok(py_false())
                }
            }
        }
    }
    fn close(&mut self) -> PyResult<()> {
        self.tx.close_channel();
        Ok(())
    }
}

#[pyclass]
struct SenderGlue {
    locals: TaskLocals,
    tx: Box<dyn Sender>,
}
#[pymethods]
impl SenderGlue {
    pub fn send(&mut self, item: PyObject) -> PyResult<PyObject> {
        self.tx.send(self.locals.clone(), item)
    }
    pub fn close(&mut self) -> PyResult<()> {
        self.tx.close()
    }
}

#[cfg(feature = "unstable-streams")]
const STREAM_GLUE: &str = r#"
import asyncio

async def forward(gen, sender):
    async for item in gen:
        should_continue = sender.send(item)

        if asyncio.iscoroutine(should_continue):
            should_continue = await should_continue

        if should_continue:
            continue
        else:
            break

    sender.close()
"#;

/// <span class="module-item stab portability" style="display: inline; border-radius: 3px; padding: 2px; font-size: 80%; line-height: 1.2;"><code>unstable-streams</code></span> Convert an async generator into a stream
///
/// **This API is marked as unstable** and is only available when the
/// `unstable-streams` crate feature is enabled. This comes with no
/// stability guarantees, and could be changed or removed at any time.
///
/// # Arguments
/// * `locals` - The current task locals
/// * `gen` - The Python async generator to be converted
///
/// # Examples
/// ```no_run
/// # use std::{task::{Context, Poll}, pin::Pin, future::Future};
/// #
/// # use pyo3_asyncio::{
/// #     TaskLocals,
/// #     generic::{JoinError, ContextExt, Runtime}
/// # };
/// #
/// # struct MyCustomJoinError;
/// #
/// # impl JoinError for MyCustomJoinError {
/// #     fn is_panic(&self) -> bool {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomJoinHandle;
/// #
/// # impl Future for MyCustomJoinHandle {
/// #     type Output = Result<(), MyCustomJoinError>;
/// #
/// #     fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomRuntime;
/// #
/// # impl Runtime for MyCustomRuntime {
/// #     type JoinError = MyCustomJoinError;
/// #     type JoinHandle = MyCustomJoinHandle;
/// #
/// #     fn spawn<F>(fut: F) -> Self::JoinHandle
/// #     where
/// #         F: Future<Output = ()> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl ContextExt for MyCustomRuntime {
/// #     fn scope<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
/// #     where
/// #         F: Future<Output = R> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// #     fn get_task_locals() -> Option<TaskLocals> {
/// #         unreachable!()
/// #     }
/// # }
///
/// use pyo3::prelude::*;
/// use futures::{StreamExt, TryStreamExt};
///
/// const TEST_MOD: &str = r#"
/// import asyncio
///
/// async def gen():
///     for i in range(10):
///         await asyncio.sleep(0.1)
///         yield i
/// "#;
///
/// # async fn test_async_gen() -> PyResult<()> {
/// let stream = Python::with_gil(|py| {
///     let test_mod = PyModule::from_code(
///         py,
///         TEST_MOD,
///         "test_rust_coroutine/test_mod.py",
///         "test_mod",
///     )?;
///
///     pyo3_asyncio::generic::into_stream_with_locals_v2::<MyCustomRuntime>(
///         pyo3_asyncio::generic::get_current_locals::<MyCustomRuntime>(py)?,
///         test_mod.call_method0("gen")?
///     )
/// })?;
///
/// let vals = stream
///     .map(|item| Python::with_gil(|py| -> PyResult<i32> { Ok(item.as_ref(py).extract()?) }))
///     .try_collect::<Vec<i32>>()
///     .await?;
///
/// assert_eq!((0..10).collect::<Vec<i32>>(), vals);
///
/// Ok(())
/// # }
/// ```
#[cfg(feature = "unstable-streams")]
pub fn into_stream_with_locals_v2<'p, R>(
    locals: TaskLocals,
    gen: &'p PyAny,
) -> PyResult<impl futures::Stream<Item = PyObject> + 'static>
where
    R: Runtime + ContextExt,
{
    static GLUE_MOD: OnceCell<PyObject> = OnceCell::new();
    let py = gen.py();
    let glue = GLUE_MOD
        .get_or_try_init(|| -> PyResult<PyObject> {
            Ok(PyModule::from_code(
                py,
                STREAM_GLUE,
                "pyo3_asyncio/pyo3_asyncio_glue.py",
                "pyo3_asyncio_glue",
            )?
            .into())
        })?
        .as_ref(py);

    let (tx, rx) = mpsc::channel(10);

    locals.event_loop(py).call_method1(
        "call_soon_threadsafe",
        (
            locals.event_loop(py).getattr("create_task")?,
            glue.call_method1(
                "forward",
                (
                    gen,
                    SenderGlue {
                        locals,
                        tx: Box::new(GenericSender {
                            runtime: PhantomData::<R>,
                            tx,
                        }),
                    },
                ),
            )?,
        ),
    )?;
    Ok(rx)
}

/// <span class="module-item stab portability" style="display: inline; border-radius: 3px; padding: 2px; font-size: 80%; line-height: 1.2;"><code>unstable-streams</code></span> Convert an async generator into a stream
///
/// **This API is marked as unstable** and is only available when the
/// `unstable-streams` crate feature is enabled. This comes with no
/// stability guarantees, and could be changed or removed at any time.
///
/// # Arguments
/// * `gen` - The Python async generator to be converted
///
/// # Examples
/// ```no_run
/// # use std::{task::{Context, Poll}, pin::Pin, future::Future};
/// #
/// # use pyo3_asyncio::{
/// #     TaskLocals,
/// #     generic::{JoinError, ContextExt, Runtime}
/// # };
/// #
/// # struct MyCustomJoinError;
/// #
/// # impl JoinError for MyCustomJoinError {
/// #     fn is_panic(&self) -> bool {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomJoinHandle;
/// #
/// # impl Future for MyCustomJoinHandle {
/// #     type Output = Result<(), MyCustomJoinError>;
/// #
/// #     fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # struct MyCustomRuntime;
/// #
/// # impl Runtime for MyCustomRuntime {
/// #     type JoinError = MyCustomJoinError;
/// #     type JoinHandle = MyCustomJoinHandle;
/// #
/// #     fn spawn<F>(fut: F) -> Self::JoinHandle
/// #     where
/// #         F: Future<Output = ()> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// # }
/// #
/// # impl ContextExt for MyCustomRuntime {
/// #     fn scope<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
/// #     where
/// #         F: Future<Output = R> + Send + 'static
/// #     {
/// #         unreachable!()
/// #     }
/// #     fn get_task_locals() -> Option<TaskLocals> {
/// #         unreachable!()
/// #     }
/// # }
///
/// use pyo3::prelude::*;
/// use futures::{StreamExt, TryStreamExt};
///
/// const TEST_MOD: &str = r#"
/// import asyncio
///
/// async def gen():
///     for i in range(10):
///         await asyncio.sleep(0.1)
///         yield i
/// "#;
///
/// # async fn test_async_gen() -> PyResult<()> {
/// let stream = Python::with_gil(|py| {
///     let test_mod = PyModule::from_code(
///         py,
///         TEST_MOD,
///         "test_rust_coroutine/test_mod.py",
///         "test_mod",
///     )?;
///
///     pyo3_asyncio::generic::into_stream_v2::<MyCustomRuntime>(test_mod.call_method0("gen")?)
/// })?;
///
/// let vals = stream
///     .map(|item| Python::with_gil(|py| -> PyResult<i32> { Ok(item.as_ref(py).extract()?) }))
///     .try_collect::<Vec<i32>>()
///     .await?;
///
/// assert_eq!((0..10).collect::<Vec<i32>>(), vals);
///
/// Ok(())
/// # }
/// ```
#[cfg(feature = "unstable-streams")]
pub fn into_stream_v2<'p, R>(
    gen: &'p PyAny,
) -> PyResult<impl futures::Stream<Item = PyObject> + 'static>
where
    R: Runtime + ContextExt,
{
    into_stream_with_locals_v2::<R>(get_current_locals::<R>(gen.py())?, gen)
}
