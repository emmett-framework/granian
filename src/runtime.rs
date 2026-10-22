use hyper::HeaderMap;
use pyo3::{IntoPyObjectExt, prelude::*};
use std::{future::Future, sync::Arc};
use tokio::{
    runtime::Builder as RuntimeBuilder,
    task::{JoinHandle, LocalSet},
};

#[cfg(unix)]
use super::callbacks::PyFutureAwaitable;
#[cfg(windows)]
use super::callbacks::{PyFutureDoneCallback, PyFutureResultSetter};

use super::blocking;
use super::callbacks::{PyDoneAwaitable, PyEmptyAwaitable, PyErrAwaitable, PyIterAwaitable};
use super::conversion::FutureResultToPy;
use super::metrics;
use super::net::SockAddr;
use super::scheduler::{FlowRef, Scheduler, SchedulerImpl};

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

    fn spawn_blocking_flow<F>(&self, task: F, _flow: Option<&FlowRef>)
    where
        F: FnOnce(Python) + Send + 'static,
    {
        self.spawn_blocking(task);
    }

    fn spawn_cancellable<F>(&self, on_cancel: Arc<tokio::sync::Notify>, fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + Send + 'static;
}

pub trait ContextExt: Runtime {
    fn py_event_loop(&self, py: Python) -> Py<PyAny>;
}

pub(crate) struct RuntimeWrapper {
    pub inner: tokio::runtime::Runtime,
    br: Arc<blocking::BlockingRunner>,
    pr: Arc<Py<PyAny>>,
    sig: Arc<tokio::sync::Notify>,
    sched: Arc<SchedulerImpl>,
}

impl RuntimeWrapper {
    pub fn new(
        blocking_threads: usize,
        py_threads: usize,
        py_threads_idle_timeout: u64,
        py_loop: Arc<Py<PyAny>>,
        metrics: Option<metrics::ArcWorkerMetrics>,
        scheduler: Option<Arc<SchedulerImpl>>,
    ) -> Self {
        let sched = scheduler.unwrap_or_else(|| Arc::new(SchedulerImpl::fifo()));
        let br = match metrics {
            Some(metrics) => blocking::BlockingRunner::new_with_metrics(
                py_threads,
                py_threads_idle_timeout,
                metrics,
                Some(sched.clone()),
            ),
            None => blocking::BlockingRunner::new(py_threads, py_threads_idle_timeout, Some(sched.clone())),
        };
        Self {
            inner: default_runtime(blocking_threads),
            br: br.into(),
            pr: py_loop,
            sig: tokio::sync::Notify::new().into(),
            sched,
        }
    }

    pub fn with_runtime(
        rt: tokio::runtime::Runtime,
        py_threads: usize,
        py_threads_idle_timeout: u64,
        py_loop: Arc<Py<PyAny>>,
        metrics: Option<metrics::ArcWorkerMetrics>,
        scheduler: Option<Arc<SchedulerImpl>>,
    ) -> Self {
        let sched = scheduler.unwrap_or_else(|| Arc::new(SchedulerImpl::fifo()));
        let br = match metrics {
            Some(metrics) => blocking::BlockingRunner::new_with_metrics(
                py_threads,
                py_threads_idle_timeout,
                metrics,
                Some(sched.clone()),
            ),
            None => blocking::BlockingRunner::new(py_threads, py_threads_idle_timeout, Some(sched.clone())),
        };
        Self {
            inner: rt,
            br: br.into(),
            pr: py_loop,
            sig: tokio::sync::Notify::new().into(),
            sched,
        }
    }

    pub fn handler(&self) -> RuntimeRef {
        RuntimeRef::new(
            self.inner.handle().clone(),
            self.br.clone(),
            self.pr.clone(),
            self.sig.clone(),
            self.sched.clone(),
        )
    }
}

#[derive(Clone)]
pub struct RuntimeRef {
    pub inner: tokio::runtime::Handle,
    innerb: Arc<blocking::BlockingRunner>,
    innerp: Arc<Py<PyAny>>,
    sig: Arc<tokio::sync::Notify>,
    sched: Arc<SchedulerImpl>,
}

impl RuntimeRef {
    pub fn new(
        rt: tokio::runtime::Handle,
        br: Arc<blocking::BlockingRunner>,
        pyloop: Arc<Py<PyAny>>,
        sig: Arc<tokio::sync::Notify>,
        sched: Arc<SchedulerImpl>,
    ) -> Self {
        Self {
            inner: rt,
            innerb: br,
            innerp: pyloop,
            sig,
            sched,
        }
    }

    pub fn close(&self) {
        self.sig.notify_waiters();
    }

    /// The scheduler shared by every runtime of the worker, used to tag events with a flow.
    #[inline]
    pub fn resolve_flow(&self, peer: &SockAddr, headers: &HeaderMap) -> Option<FlowRef> {
        self.sched.resolve(peer, headers)
    }

    #[inline]
    pub fn release_flow(&self, flow: FlowRef) {
        self.sched.release(flow);
    }

    #[inline]
    pub fn debit_flow(&self, flow: &FlowRef, cost_ns: u64) {
        self.sched.debit(flow, cost_ns);
    }

    #[inline]
    pub fn lane_enqueue(&self, flow: &FlowRef, fut: Py<PyAny>) -> bool {
        self.sched.lane_enqueue(flow, fut)
    }

    #[inline]
    pub fn lane_next(&self) -> Option<Py<PyAny>> {
        self.sched.lane_next()
    }

    #[inline]
    pub fn lane_release(&self) -> bool {
        self.sched.lane_release()
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

    #[inline]
    fn spawn_blocking_flow<F>(&self, task: F, flow: Option<&FlowRef>)
    where
        F: FnOnce(Python) + Send + 'static,
    {
        _ = self.innerb.run_with_flow(task, flow);
    }

    fn spawn_cancellable<F>(&self, on_cancel: Arc<tokio::sync::Notify>, fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let sig = self.sig.clone();

        self.inner.spawn(async move {
            tokio::select! {
                biased;
                () = fut => {},
                () = sig.notified() => {
                    on_cancel.notify_one();
                }
            };
        })
    }
}

impl ContextExt for RuntimeRef {
    fn py_event_loop(&self, py: Python) -> Py<PyAny> {
        self.innerp.clone_ref(py)
    }
}

/// Per-request runtime handle. It behaves like `RuntimeRef`, but tags every event it
/// pushes to the Python side with the request's flow, so that the fair scheduler can
/// order them.
#[derive(Clone)]
pub(crate) struct RequestRt {
    pub rt: RuntimeRef,
    pub flow: Option<FlowRef>,
}

impl RequestRt {
    pub fn new(rt: RuntimeRef, flow: Option<FlowRef>) -> Self {
        Self { rt, flow }
    }

    /// Release the request's flow, if any.
    pub fn release(&self) {
        if let Some(flow) = self.flow {
            self.rt.release_flow(flow);
        }
    }
}

impl Runtime for RequestRt {
    type JoinError = <RuntimeRef as Runtime>::JoinError;
    type JoinHandle = <RuntimeRef as Runtime>::JoinHandle;

    fn spawn<F>(&self, fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.rt.spawn(fut)
    }

    fn spawn_blocking<F>(&self, task: F)
    where
        F: FnOnce(Python) + Send + 'static,
    {
        self.rt.spawn_blocking_flow(task, self.flow.as_ref());
    }

    fn spawn_cancellable<F>(&self, on_cancel: Arc<tokio::sync::Notify>, fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.rt.spawn_cancellable(on_cancel, fut)
    }
}

impl ContextExt for RequestRt {
    fn py_event_loop(&self, py: Python) -> Py<PyAny> {
        self.rt.py_event_loop(py)
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
    py_loop: Arc<Py<PyAny>>,
    metrics: Option<metrics::ArcWorkerMetrics>,
    scheduler: Option<Arc<SchedulerImpl>>,
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
        metrics,
        scheduler,
    )
}

pub(crate) fn init_runtime_st(
    blocking_threads: usize,
    py_threads: usize,
    py_threads_idle_timeout: u64,
    py_loop: Arc<Py<PyAny>>,
    metrics: Option<metrics::ArcWorkerMetrics>,
    scheduler: Option<Arc<SchedulerImpl>>,
) -> RuntimeWrapper {
    RuntimeWrapper::new(
        blocking_threads,
        py_threads,
        py_threads_idle_timeout,
        py_loop,
        metrics,
        scheduler,
    )
}

#[inline(always)]
pub(crate) fn empty_future_into_py(py: Python) -> PyResult<Bound<PyAny>> {
    PyEmptyAwaitable.into_bound_py_any(py)
}

#[inline(always)]
pub(crate) fn done_future_into_py(py: Python, result: PyResult<Py<PyAny>>) -> PyResult<Bound<PyAny>> {
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
pub(crate) fn future_into_py_iter<R, F>(rt: R, py: Python<'_>, fut: F) -> PyResult<Bound<'_, PyAny>>
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
pub(crate) fn future_into_py_futlike<R, F>(rt: R, py: Python<'_>, fut: F) -> PyResult<Bound<'_, PyAny>>
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
            biased;
            result = fut => rth.spawn_blocking(move |py| PyFutureAwaitable::set_result(aw, py, result)),
            () = cancel_tx.notified() => rth.spawn_blocking(move |py| aw.drop_ref(py)),
        }
    });

    Ok(py_fut.into_any().into_bound(py))
}

#[allow(unused_must_use)]
#[cfg(windows)]
pub(crate) fn future_into_py_futlike<R, F>(rt: R, py: Python<'_>, fut: F) -> PyResult<Bound<'_, PyAny>>
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
            biased;
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

pub(crate) fn block_on_local<F>(rt: &RuntimeWrapper, local: LocalSet, fut: F)
where
    F: Future + 'static,
{
    local.block_on(&rt.inner, fut);
}
