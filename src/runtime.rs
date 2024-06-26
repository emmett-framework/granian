use pyo3::prelude::*;
use std::{
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::{
    runtime::Builder as RuntimeBuilder,
    task::{JoinHandle, LocalSet},
};

use super::blocking::BlockingRunner;
use super::callbacks::PyEmptyAwaitable;
#[cfg(unix)]
use super::callbacks::PyFutureAwaitable;
#[cfg(not(target_os = "linux"))]
use super::callbacks::PyIterAwaitable;
#[cfg(windows)]
use super::callbacks::{PyFutureDoneCallback, PyFutureResultSetter};

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

    fn blocking(&self) -> BlockingRunner;
}

pub trait ContextExt: Runtime {
    fn py_event_loop(&self, py: Python) -> PyObject;
}

pub(crate) struct RuntimeWrapper {
    rt: tokio::runtime::Runtime,
    br: BlockingRunner,
    pr: Arc<PyObject>,
}

impl RuntimeWrapper {
    pub fn new(blocking_threads: usize, py_loop: Arc<PyObject>) -> Self {
        Self {
            rt: default_runtime(blocking_threads),
            br: BlockingRunner::new(),
            pr: py_loop,
        }
    }

    pub fn with_runtime(rt: tokio::runtime::Runtime, py_loop: Arc<PyObject>) -> Self {
        Self {
            rt,
            br: BlockingRunner::new(),
            pr: py_loop,
        }
    }

    pub fn handler(&self) -> RuntimeRef {
        RuntimeRef::new(self.rt.handle().clone(), self.br.clone(), self.pr.clone())
    }
}

#[derive(Clone)]
pub struct RuntimeRef {
    pub inner: tokio::runtime::Handle,
    pub innerb: BlockingRunner,
    innerp: Arc<PyObject>,
}

impl RuntimeRef {
    pub fn new(rt: tokio::runtime::Handle, br: BlockingRunner, pyloop: Arc<PyObject>) -> Self {
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

    fn blocking(&self) -> BlockingRunner {
        self.innerb.clone()
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

pub(crate) fn init_runtime_mt(threads: usize, blocking_threads: usize, py_loop: Arc<PyObject>) -> RuntimeWrapper {
    RuntimeWrapper::with_runtime(
        RuntimeBuilder::new_multi_thread()
            .worker_threads(threads)
            .max_blocking_threads(blocking_threads)
            .enable_all()
            .build()
            .unwrap(),
        py_loop,
    )
}

pub(crate) fn init_runtime_st(blocking_threads: usize, py_loop: Arc<PyObject>) -> RuntimeWrapper {
    RuntimeWrapper::new(blocking_threads, py_loop)
}

// NOTE:
//  `future_into_py_iter` relies on what CPython refers as "bare yield".
//  This is generally ~55% faster than `pyo3_asyncio.future_into_py` implementation.
//  It consumes more cpu-cycles than `future_into_py_futlike`,
//  but for "quick" operations it's something like 12% faster.
#[allow(unused_must_use)]
#[cfg(not(target_os = "linux"))]
pub(crate) fn future_into_py_iter<R, F, T>(rt: R, py: Python, fut: F) -> PyResult<Bound<PyAny>>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPy<PyObject> + Send + 'static,
{
    let aw = Py::new(py, PyIterAwaitable::new())?;
    let py_fut = aw.clone_ref(py);
    let rb = rt.blocking();

    rt.spawn(async move {
        let result = fut.await;
        let _ = rb.run(move || {
            aw.get().set_result(result);
            Python::with_gil(|_| drop(aw));
        });
    });

    Ok(py_fut.into_any().into_bound(py))
}

// NOTE:
//  for some unknown reasons, it seems on Linux the real implementation
//  has performance issues. We just fallback to `futlike` impl on such targets.
//  MacOS works best with original impl, Windows still needs further analysis.
#[cfg(target_os = "linux")]
#[inline(always)]
pub(crate) fn future_into_py_iter<R, F, T>(rt: R, py: Python, fut: F) -> PyResult<Bound<PyAny>>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPy<PyObject> + Send + 'static,
{
    future_into_py_futlike(rt, py, fut)
}

// NOTE:
//  `future_into_py_futlike` relies on an `asyncio.Future` like implementation.
//  This is generally ~38% faster than `pyo3_asyncio.future_into_py` implementation.
//  It won't consume more cpu-cycles than standard asyncio implementation,
//  and for "long" operations it's something like 6% faster than `future_into_py_iter`.
#[allow(unused_must_use)]
#[cfg(unix)]
pub(crate) fn future_into_py_futlike<R, F, T>(rt: R, py: Python, fut: F) -> PyResult<Bound<PyAny>>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPy<PyObject> + Send + 'static,
{
    let event_loop = rt.py_event_loop(py);
    let (aw, cancel_tx) = PyFutureAwaitable::new(event_loop).to_spawn(py)?;
    let aw_ref = aw.clone_ref(py);
    let py_fut = aw.clone_ref(py);
    let rb = rt.blocking();

    rt.spawn(async move {
        tokio::select! {
            result = fut => {
                let _ = rb.run(move || {
                    aw.get().set_result(result, aw_ref);
                    Python::with_gil(|_| drop(aw));
                });
            },
            () = cancel_tx.notified() => {}
        }
    });

    Ok(py_fut.into_any().into_bound(py))
}

#[allow(unused_must_use)]
#[cfg(windows)]
pub(crate) fn future_into_py_futlike<R, F, T>(rt: R, py: Python, fut: F) -> PyResult<Bound<PyAny>>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPy<PyObject> + Send + 'static,
{
    let event_loop = rt.py_event_loop(py);
    let event_loop_ref = event_loop.clone_ref(py);
    let cancel_tx = Arc::new(tokio::sync::Notify::new());
    let rb = rt.blocking();

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
                let _ = rb.run(move || {
                    Python::with_gil(|py| {
                        let (cb, value) = match result {
                            Ok(val) => (fut_ref.getattr(py, pyo3::intern!(py, "set_result")).unwrap(), val.into_py(py)),
                            Err(err) => (fut_ref.getattr(py, pyo3::intern!(py, "set_exception")).unwrap(), err.into_py(py))
                        };
                        let _ = event_loop_ref.call_method1(py, pyo3::intern!(py, "call_soon_threadsafe"), (PyFutureResultSetter, cb, value));
                        drop(fut_ref);
                        drop(event_loop_ref);
                    });
                });
            },
            () = cancel_tx.notified() => {}
        }
    });

    Ok(py_fut.into_bound(py))
}

#[allow(clippy::unnecessary_wraps)]
#[inline(always)]
pub(crate) fn empty_future_into_py(py: Python) -> PyResult<Bound<PyAny>> {
    Ok(PyEmptyAwaitable.into_py(py).into_bound(py))
}

#[allow(unused_must_use)]
pub(crate) fn run_until_complete<R, F, T>(rt: R, event_loop: Bound<PyAny>, fut: F) -> PyResult<T>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: Send + Sync + 'static,
{
    let py = event_loop.py();
    let result_tx = Arc::new(Mutex::new(None));
    let result_rx = Arc::clone(&result_tx);

    let py_fut = event_loop.call_method0("create_future")?;
    let loop_tx = event_loop.clone().into_py(py);
    let future_tx = py_fut.clone().into_py(py);

    rt.spawn(async move {
        let val = fut.await;
        if let Ok(mut result) = result_tx.lock() {
            *result = Some(val.unwrap());
        }

        // NOTE: we don't care if we block the runtime.
        //       `run_until_complete` is used only for the workers main loop.
        Python::with_gil(move |py| {
            let res_method = future_tx.getattr(py, "set_result").unwrap();
            let _ = loop_tx.call_method_bound(py, "call_soon_threadsafe", (res_method, py.None()), None);
            drop(future_tx);
            drop(loop_tx);
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
