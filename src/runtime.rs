use pyo3::{prelude::*, IntoPyObjectExt};
use std::{
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::{
    runtime::Builder as RuntimeBuilder,
    task::{JoinHandle, LocalSet},
};

#[cfg(unix)]
use super::callbacks::PyFutureAwaitable;
#[cfg(windows)]
use super::callbacks::{PyFutureDoneCallback, PyFutureResultSetter};

use super::blocking::BlockingRunner;
use super::callbacks::{PyDoneAwaitable, PyEmptyAwaitable, PyErrAwaitable, PyIterAwaitable};
use super::conversion::FutureResultToPy;

pub trait JoinError {
    #[allow(dead_code)]
    fn is_panic(&self) -> bool;
}

pub trait Runtime: Send + 'static {
    type JoinError: JoinError + Send;
    type JoinHandle: Future<Output = Result<(), Self::JoinError>> + Send;

    fn spawn<F>(&self, fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + Send + 'static;

    fn spawn_blocking<F>(&self, task: F)
    where
        F: FnOnce(Python) + Send + 'static;
}

pub trait ContextExt: Runtime {
    fn py_event_loop(&self, py: Python) -> PyObject;
}

pub(crate) struct RuntimeWrapper {
    pub inner: tokio::runtime::Runtime,
    br: Arc<BlockingRunner>,
    pr: Arc<PyObject>,
}

impl RuntimeWrapper {
    pub fn new(
        blocking_threads: usize,
        py_threads: usize,
        py_threads_idle_timeout: u64,
        py_loop: Arc<PyObject>,
    ) -> Self {
        Self {
            inner: default_runtime(blocking_threads),
            br: BlockingRunner::new(py_threads, py_threads_idle_timeout).into(),
            pr: py_loop,
        }
    }

    pub fn with_runtime(
        rt: tokio::runtime::Runtime,
        py_threads: usize,
        py_threads_idle_timeout: u64,
        py_loop: Arc<PyObject>,
    ) -> Self {
        Self {
            inner: rt,
            br: BlockingRunner::new(py_threads, py_threads_idle_timeout).into(),
            pr: py_loop,
        }
    }

    pub fn handler(&self) -> RuntimeRef {
        RuntimeRef::new(self.inner.handle().clone(), self.br.clone(), self.pr.clone())
    }
}

#[derive(Clone)]
pub struct RuntimeRef {
    pub inner: tokio::runtime::Handle,
    innerb: Arc<BlockingRunner>,
    innerp: Arc<PyObject>,
}

impl RuntimeRef {
    pub fn new(rt: tokio::runtime::Handle, br: Arc<BlockingRunner>, pyloop: Arc<PyObject>) -> Self {
        Self {
            inner: rt,
            innerb: br,
            innerp: pyloop,
        }
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

    #[inline]
    fn spawn_blocking<F>(&self, task: F)
    where
        F: FnOnce(Python) + Send + 'static,
    {
        _ = self.innerb.run(task);
    }
}

impl ContextExt for RuntimeRef {
    fn py_event_loop(&self, py: Python) -> PyObject {
        self.innerp.clone_ref(py)
    }
}

fn default_runtime(blocking_threads: usize) -> tokio::runtime::Runtime {
    RuntimeBuilder::new_current_thread()
        .max_blocking_threads(blocking_threads)
        .enable_all()
        .build()
        .unwrap()
}

pub(crate) fn init_runtime_mt(
    threads: usize,
    blocking_threads: usize,
    py_threads: usize,
    py_threads_idle_timeout: u64,
    py_loop: Arc<PyObject>,
) -> RuntimeWrapper {
    RuntimeWrapper::with_runtime(
        RuntimeBuilder::new_multi_thread()
            .worker_threads(threads)
            .max_blocking_threads(blocking_threads)
            .enable_all()
            .build()
            .unwrap(),
        py_threads,
        py_threads_idle_timeout,
        py_loop,
    )
}

pub(crate) fn init_runtime_st(
    blocking_threads: usize,
    py_threads: usize,
    py_threads_idle_timeout: u64,
    py_loop: Arc<PyObject>,
) -> RuntimeWrapper {
    RuntimeWrapper::new(blocking_threads, py_threads, py_threads_idle_timeout, py_loop)
}

#[inline(always)]
pub(crate) fn empty_future_into_py(py: Python) -> PyResult<Bound<PyAny>> {
    PyEmptyAwaitable.into_bound_py_any(py)
}

#[inline(always)]
pub(crate) fn done_future_into_py(py: Python, result: PyResult<PyObject>) -> PyResult<Bound<PyAny>> {
    PyDoneAwaitable::new(result).into_bound_py_any(py)
}

#[inline(always)]
pub(crate) fn err_future_into_py(py: Python, err: PyResult<()>) -> PyResult<Bound<PyAny>> {
    PyErrAwaitable::new(err).into_bound_py_any(py)
}

// NOTE:
//  `future_into_py_iter` relies on what CPython refers as "bare yield".
//  This is generally ~55% faster than `pyo3_asyncio.future_into_py` implementation.
//  It consumes more cpu-cycles than `future_into_py_futlike`,
//  but for "quick" operations it's something like 12% faster.
#[allow(dead_code, unused_must_use)]
pub(crate) fn future_into_py_iter<R, F>(rt: R, py: Python, fut: F) -> PyResult<Bound<PyAny>>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = FutureResultToPy> + Send + 'static,
{
    let aw = Py::new(py, PyIterAwaitable::new())?;
    let py_fut = aw.clone_ref(py);
    let rth = rt.clone();

    rt.spawn(async move {
        let result = fut.await;
        rth.spawn_blocking(move |py| PyIterAwaitable::set_result(aw, py, result));
    });

    Ok(py_fut.into_any().into_bound(py))
}

// NOTE:
//  `future_into_py_futlike` relies on an `asyncio.Future` like implementation.
//  This is generally ~38% faster than `pyo3_asyncio.future_into_py` implementation.
//  It won't consume more cpu-cycles than standard asyncio implementation,
//  and for "long" operations it's something like 6% faster than `future_into_py_iter`.
#[allow(unused_must_use)]
#[cfg(unix)]
pub(crate) fn future_into_py_futlike<R, F>(rt: R, py: Python, fut: F) -> PyResult<Bound<PyAny>>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = FutureResultToPy> + Send + 'static,
{
    let event_loop = rt.py_event_loop(py);
    let (aw, cancel_tx) = PyFutureAwaitable::new(event_loop).to_spawn(py)?;
    let py_fut = aw.clone_ref(py);
    let rth = rt.clone();

    rt.spawn(async move {
        tokio::select! {
            result = fut => rth.spawn_blocking(move |py| PyFutureAwaitable::set_result(aw, py, result)),
            () = cancel_tx.notified() => rth.spawn_blocking(move |py| aw.drop_ref(py)),
        }
    });

    Ok(py_fut.into_any().into_bound(py))
}

#[allow(unused_must_use)]
#[cfg(windows)]
pub(crate) fn future_into_py_futlike<R, F>(rt: R, py: Python, fut: F) -> PyResult<Bound<PyAny>>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = FutureResultToPy> + Send + 'static,
{
    let event_loop = rt.py_event_loop(py);
    let event_loop_ref = event_loop.clone_ref(py);
    let cancel_tx = Arc::new(tokio::sync::Notify::new());
    let rth = rt.clone();

    let py_fut = event_loop.call_method0(py, pyo3::intern!(py, "create_future"))?;
    py_fut.call_method1(
        py,
        pyo3::intern!(py, "add_done_callback"),
        (PyFutureDoneCallback {
            cancel_tx: cancel_tx.clone(),
        },),
    )?;
    let fut_ref = py_fut.clone_ref(py);

    rt.spawn(async move {
        tokio::select! {
            result = fut => {
                rth.spawn_blocking(move |py| {
                    let pyres = result.into_pyobject(py).map(Bound::unbind);
                    let (cb, value) = match pyres {
                        Ok(val) => (fut_ref.getattr(py, pyo3::intern!(py, "set_result")).unwrap(), val),
                        Err(err) => (fut_ref.getattr(py, pyo3::intern!(py, "set_exception")).unwrap(), err.into_py_any(py).unwrap())
                    };
                    let _ = event_loop_ref.call_method1(py, pyo3::intern!(py, "call_soon_threadsafe"), (PyFutureResultSetter, cb, value));
                    fut_ref.drop_ref(py);
                    event_loop_ref.drop_ref(py);
                });
            },
            () = cancel_tx.notified() => {
                rth.spawn_blocking(move |py| {
                    fut_ref.drop_ref(py);
                    event_loop_ref.drop_ref(py);
                });
            }
        }
    });

    Ok(py_fut.into_bound(py))
}

#[allow(unused_must_use)]
pub(crate) fn run_until_complete<F>(rt: RuntimeWrapper, event_loop: Bound<PyAny>, fut: F) -> PyResult<()>
where
    F: Future<Output = PyResult<()>> + Send + 'static,
{
    let result_tx = Arc::new(Mutex::new(None));
    let result_rx = Arc::clone(&result_tx);

    let py_fut = event_loop.call_method0("create_future")?;
    let loop_tx = event_loop.clone().unbind();
    let future_tx = py_fut.clone().unbind();

    rt.inner.spawn(async move {
        let _ = fut.await;
        if let Ok(mut result) = result_tx.lock() {
            *result = Some(());
        }

        // NOTE: we don't care if we block the runtime.
        //       `run_until_complete` is used only for the workers main loop.
        Python::with_gil(move |py| {
            let res_method = future_tx.getattr(py, "set_result").unwrap();
            let _ = loop_tx.call_method(py, "call_soon_threadsafe", (res_method, py.None()), None);
            future_tx.drop_ref(py);
            loop_tx.drop_ref(py);
        });
    });

    event_loop.call_method1("run_until_complete", (py_fut,))?;

    result_rx.lock().unwrap().take().unwrap();
    Ok(())
}

pub(crate) fn block_on_local<F>(rt: &RuntimeWrapper, local: LocalSet, fut: F)
where
    F: Future + 'static,
{
    local.block_on(&rt.inner, fut);
}
