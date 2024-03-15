use pyo3::prelude::*;
use pyo3_asyncio::TaskLocals;
use std::{
    cell::OnceCell,
    future::Future,
    io,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::{
    runtime::Builder,
    task::{JoinHandle, LocalSet},
};

#[cfg(unix)]
use super::callbacks::PyFutureAwaitable;
use super::callbacks::{PyEmptyAwaitable, PyIterAwaitable};
#[cfg(windows)]
use super::callbacks::{PyFutureDoneCallback, PyFutureResultSetter};

tokio::task_local! {
    static TASK_LOCALS: OnceCell<TaskLocals>;
}

pub trait JoinError {
    fn is_panic(&self) -> bool;
}

pub trait Runtime: Send + 'static {
    type JoinError: JoinError + Send;
    type JoinHandle: Future<Output = Result<(), Self::JoinError>> + Send;

    fn spawn<F>(&self, fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + Send + 'static;

    fn handler(&self) -> RuntimeRef;
}

pub trait ContextExt: Runtime {
    fn scope<F, R>(&self, locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
    where
        F: Future<Output = R> + Send + 'static;

    fn get_task_locals() -> Option<TaskLocals>;
}

pub trait SpawnLocalExt: Runtime {
    fn spawn_local<F>(&self, fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + 'static;
}

pub trait LocalContextExt: Runtime {
    fn scope_local<F, R>(&self, locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R>>>
    where
        F: Future<Output = R> + 'static;
}

pub(crate) struct RuntimeWrapper {
    rt: tokio::runtime::Runtime,
}

impl RuntimeWrapper {
    pub fn new(blocking_threads: usize) -> Self {
        Self {
            rt: default_runtime(blocking_threads).unwrap(),
        }
    }

    pub fn with_runtime(rt: tokio::runtime::Runtime) -> Self {
        Self { rt }
    }

    pub fn handler(&self) -> RuntimeRef {
        RuntimeRef::new(self.rt.handle().clone())
    }
}

#[derive(Clone)]
pub struct RuntimeRef {
    pub inner: tokio::runtime::Handle,
}

impl RuntimeRef {
    pub fn new(rt: tokio::runtime::Handle) -> Self {
        Self { inner: rt }
    }
}

impl JoinError for tokio::task::JoinError {
    fn is_panic(&self) -> bool {
        tokio::task::JoinError::is_panic(self)
    }
}

impl Runtime for RuntimeRef {
    type JoinError = tokio::task::JoinError;
    type JoinHandle = JoinHandle<()>;

    fn spawn<F>(&self, fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.inner.spawn(fut)
    }

    fn handler(&self) -> RuntimeRef {
        RuntimeRef::new(self.inner.clone())
    }
}

impl ContextExt for RuntimeRef {
    fn scope<F, R>(&self, locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
    where
        F: Future<Output = R> + Send + 'static,
    {
        let cell = OnceCell::new();
        cell.set(locals).unwrap();

        Box::pin(TASK_LOCALS.scope(cell, fut))
    }

    fn get_task_locals() -> Option<TaskLocals> {
        match TASK_LOCALS.try_with(|c| c.get().cloned()) {
            Ok(locals) => locals,
            Err(_) => None,
        }
    }
}

impl SpawnLocalExt for RuntimeRef {
    fn spawn_local<F>(&self, fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + 'static,
    {
        tokio::task::spawn_local(fut)
    }
}

impl LocalContextExt for RuntimeRef {
    fn scope_local<F, R>(&self, locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R>>>
    where
        F: Future<Output = R> + 'static,
    {
        let cell = OnceCell::new();
        cell.set(locals).unwrap();

        Box::pin(TASK_LOCALS.scope(cell, fut))
    }
}

fn default_runtime(blocking_threads: usize) -> io::Result<tokio::runtime::Runtime> {
    Builder::new_current_thread()
        .max_blocking_threads(blocking_threads)
        .enable_all()
        .build()
}

pub(crate) fn init_runtime_mt(threads: usize, blocking_threads: usize) -> RuntimeWrapper {
    RuntimeWrapper::with_runtime(
        Builder::new_multi_thread()
            .worker_threads(threads)
            .max_blocking_threads(blocking_threads)
            .enable_all()
            .build()
            .unwrap(),
    )
}

pub(crate) fn init_runtime_st(blocking_threads: usize) -> RuntimeWrapper {
    RuntimeWrapper::new(blocking_threads)
}

// pub(crate) fn into_future(awaitable: &PyAny) -> PyResult<impl Future<Output = PyResult<PyObject>> + Send> {
//     pyo3_asyncio::into_future_with_locals(&get_current_locals::<RuntimeRef>(awaitable.py())?, awaitable)
// }

#[inline]
fn get_current_locals<R>(py: Python) -> PyResult<TaskLocals>
where
    R: ContextExt,
{
    if let Some(locals) = R::get_task_locals() {
        Ok(locals)
    } else {
        Ok(TaskLocals::with_running_loop(py)?.copy_context(py)?)
    }
}

// NOTE:
//  `future_into_py_iter` relies on what CPython refers as "bare yield".
//  This is generally ~55% faster than `pyo3_asyncio.future_into_py` implementation.
//  It consumes more cpu-cycles than `future_into_py_futlike`,
//  but for "quick" operations it's something like 12% faster.
#[allow(unused_must_use)]
pub(crate) fn future_into_py_iter<R, F, T>(rt: R, py: Python, fut: F) -> PyResult<&PyAny>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPy<PyObject>,
{
    let aw = Py::new(py, PyIterAwaitable::new())?;
    let py_fut = aw.clone_ref(py);

    rt.spawn(async move {
        let result = fut.await;
        aw.get().set_result(result);
    });

    Ok(py_fut.into_ref(py))
}

// NOTE:
//  `future_into_py_futlike` relies on an `asyncio.Future` like implementation.
//  This is generally ~38% faster than `pyo3_asyncio.future_into_py` implementation.
//  It won't consume more cpu-cycles than standard asyncio implementation,
//  and for "long" operations it's something like 6% faster than `future_into_py_iter`.
#[allow(unused_must_use)]
#[cfg(unix)]
pub(crate) fn future_into_py_futlike<R, F, T>(rt: R, py: Python, fut: F) -> PyResult<&PyAny>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPy<PyObject>,
{
    let task_locals = get_current_locals::<R>(py)?;
    let event_loop = task_locals.event_loop(py).to_object(py);
    let (aw, cancel_tx) = PyFutureAwaitable::new(event_loop).to_spawn(py)?;
    let aw_ref = aw.clone_ref(py);
    let py_fut = aw.clone_ref(py);

    rt.spawn(async move {
        tokio::select! {
            result = fut => aw.get().set_result(result, aw_ref),
            () = cancel_tx.notified() => {}
        }
    });

    Ok(py_fut.into_ref(py))
}

#[allow(unused_must_use)]
#[cfg(windows)]
pub(crate) fn future_into_py_futlike<R, F, T>(rt: R, py: Python, fut: F) -> PyResult<&PyAny>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPy<PyObject>,
{
    let task_locals = get_current_locals::<R>(py)?;
    let event_loop = task_locals.event_loop(py);
    let event_loop_ref = event_loop.to_object(py);
    let cancel_tx = Arc::new(tokio::sync::Notify::new());

    let py_fut = event_loop.call_method0(pyo3::intern!(py, "create_future"))?;
    py_fut.call_method1(
        pyo3::intern!(py, "add_done_callback"),
        (PyFutureDoneCallback {
            cancel_tx: cancel_tx.clone(),
        },),
    )?;
    let fut_ref = PyObject::from(py_fut);

    rt.spawn(async move {
        tokio::select! {
            result = fut => {
                Python::with_gil(|py| {
                    let (cb, value) = match result {
                        Ok(val) => (fut_ref.getattr(py, pyo3::intern!(py, "set_result")).unwrap(), val.into_py(py)),
                        Err(err) => (fut_ref.getattr(py, pyo3::intern!(py, "set_exception")).unwrap(), err.into_py(py))
                    };
                    let _ = event_loop_ref.call_method1(py, pyo3::intern!(py, "call_soon_threadsafe"), (PyFutureResultSetter, cb, value));
                });
            },
            () = cancel_tx.notified() => {}
        }
    });

    Ok(py_fut)
}

#[allow(clippy::unnecessary_wraps)]
#[inline(always)]
pub(crate) fn empty_future_into_py(py: Python) -> PyResult<&PyAny> {
    Ok(PyEmptyAwaitable.into_py(py).into_ref(py))
}

#[allow(unused_must_use)]
pub(crate) fn run_until_complete<R, F, T>(rt: R, event_loop: &PyAny, fut: F) -> PyResult<T>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: Send + Sync + 'static,
{
    let py = event_loop.py();
    let result_tx = Arc::new(Mutex::new(None));
    let result_rx = Arc::clone(&result_tx);

    let task_locals = TaskLocals::new(event_loop).copy_context(py)?;
    let py_fut = event_loop.call_method0("create_future")?;
    let loop_tx = event_loop.into_py(py);
    let future_tx = py_fut.into_py(py);

    let rth = rt.handler();

    rt.spawn(async move {
        let val = rth.scope(task_locals.clone(), fut).await;
        if let Ok(mut result) = result_tx.lock() {
            *result = Some(val.unwrap());
        }

        Python::with_gil(move |py| {
            let res_method = future_tx.getattr(py, "set_result").unwrap();
            let _ = loop_tx.call_method(py, "call_soon_threadsafe", (res_method, py.None()), None);
        });
    });

    event_loop.call_method1("run_until_complete", (py_fut,))?;

    let result = result_rx.lock().unwrap().take().unwrap();
    Ok(result)
}

pub(crate) fn block_on_local<F>(rt: RuntimeWrapper, local: LocalSet, fut: F)
where
    F: Future + 'static,
{
    local.block_on(&rt.rt, fut);
}
