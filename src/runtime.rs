use once_cell::{unsync::OnceCell as UnsyncOnceCell};
use pyo3_asyncio::{TaskLocals, generic as pyrt_generic};
use pyo3::prelude::*;
use std::{future::Future, io, pin::Pin, sync::{Arc, Mutex}};
use tokio::{runtime::Builder, task::{JoinHandle, LocalSet}};

tokio::task_local! {
    static TASK_LOCALS: UnsyncOnceCell<TaskLocals>;
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
    fn scope<F, R>(
        &self,
        locals: TaskLocals,
        fut: F
    ) -> Pin<Box<dyn Future<Output = R> + Send>>
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
    fn scope_local<F, R>(
        &self,
        locals: TaskLocals,
        fut: F
    ) -> Pin<Box<dyn Future<Output = R>>>
    where
        F: Future<Output = R> + 'static;
}

pub(crate) struct RuntimeWrapper {
    rt: tokio::runtime::Runtime
}

impl RuntimeWrapper {
    pub fn new() -> Self {
        Self { rt: default_runtime().unwrap() }
    }

    pub fn with_runtime(rt: tokio::runtime::Runtime) -> Self {
        Self { rt: rt }
    }

    pub fn handler(&self) -> RuntimeRef {
        RuntimeRef::new(self.rt.handle().to_owned())
    }
}

#[derive(Clone)]
pub struct RuntimeRef {
    pub inner: tokio::runtime::Handle
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
        self.inner.spawn(async move {
            fut.await;
        })
    }

    fn handler(&self) -> RuntimeRef {
        RuntimeRef::new(self.inner.clone())
    }
}

impl ContextExt for RuntimeRef {
    fn scope<F, R>(
        &self,
        locals: TaskLocals,
        fut: F
    ) -> Pin<Box<dyn Future<Output = R> + Send>>
    where
        F: Future<Output = R> + Send + 'static,
    {
        let cell = UnsyncOnceCell::new();
        cell.set(locals).unwrap();

        Box::pin(TASK_LOCALS.scope(cell, fut))
    }

    fn get_task_locals() -> Option<TaskLocals> {
        match TASK_LOCALS.try_with(|c| c.get().map(|locals| locals.clone())) {
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
    fn scope_local<F, R>(
        &self,
        locals: TaskLocals,
        fut: F
    ) -> Pin<Box<dyn Future<Output = R>>>
    where
        F: Future<Output = R> + 'static,
    {
        let cell = UnsyncOnceCell::new();
        cell.set(locals).unwrap();

        Box::pin(TASK_LOCALS.scope(cell, fut))
    }
}

fn default_runtime() -> io::Result<tokio::runtime::Runtime> {
    Builder::new_current_thread()
        .enable_all()
        .build()
}

pub(crate) fn init_runtime_mt(threads: usize) -> RuntimeWrapper {
    RuntimeWrapper::with_runtime(
        Builder::new_multi_thread()
            .worker_threads(threads)
            .enable_all()
            .build()
            .unwrap()
    )
}

pub(crate) fn init_runtime_st() -> RuntimeWrapper {
    RuntimeWrapper::new()
}

pub(crate) fn into_future(
    awaitable: &PyAny,
) -> PyResult<impl Future<Output = PyResult<PyObject>> + Send> {
    pyo3_asyncio::into_future_with_locals(
        &get_current_locals::<RuntimeRef>(awaitable.py())?, awaitable
    )
}

pub(crate) fn future_into_py_with_locals<R, F, T>(
    rt: R,
    py: Python,
    locals: TaskLocals,
    fut: F,
) -> PyResult<&PyAny>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPy<PyObject>,
{
    let (cancel_tx, cancel_rx) = futures::channel::oneshot::channel();

    let py_fut = pyo3_asyncio::create_future(locals.event_loop(py))?;
    py_fut.call_method1(
        "add_done_callback",
        (pyrt_generic::PyDoneCallback {
            cancel_tx: Some(cancel_tx),
        },),
    )?;

    let future_tx1 = PyObject::from(py_fut);
    let future_tx2 = future_tx1.clone();
    let rth = rt.handler();

    rt.spawn(async move {
        let rti = rth.handler();
        let locals2 = locals.clone();

        if let Err(e) = rth.spawn(async move {
            let result = rti.scope(
                locals2.clone(),
                pyrt_generic::Cancellable::new_with_cancel_rx(fut, cancel_rx),
            )
            .await;

            Python::with_gil(move |py| {
                if pyrt_generic::cancelled(future_tx1.as_ref(py))
                    .map_err(pyo3_asyncio::dump_err(py))
                    .unwrap_or(false)
                {
                    return;
                }

                let _ = pyrt_generic::set_result(
                    locals2.event_loop(py),
                    future_tx1.as_ref(py),
                    result.map(|val| val.into_py(py)),
                )
                .map_err(pyo3_asyncio::dump_err(py));
            });
        })
        .await
        {
            if e.is_panic() {
                Python::with_gil(move |py| {
                    if pyrt_generic::cancelled(future_tx2.as_ref(py))
                        .map_err(pyo3_asyncio::dump_err(py))
                        .unwrap_or(false)
                    {
                        return;
                    }

                    let _ = pyrt_generic::set_result(
                        locals.event_loop(py),
                        future_tx2.as_ref(py),
                        Err(pyo3_asyncio::err::RustPanic::new_err("rust future panicked")),
                    )
                    .map_err(pyo3_asyncio::dump_err(py));
                });
            }
        }
    });

    Ok(py_fut)
}

pub fn local_future_into_py_with_locals<R, F, T>(
    rt: R,
    py: Python,
    locals: TaskLocals,
    fut: F,
) -> PyResult<&PyAny>
where
    R: Runtime + SpawnLocalExt + LocalContextExt,
    F: Future<Output = PyResult<T>> + 'static,
    T: IntoPy<PyObject>,
{
    let (cancel_tx, cancel_rx) = futures::channel::oneshot::channel();

    let py_fut = pyo3_asyncio::create_future(locals.event_loop(py))?;
    py_fut.call_method1(
        "add_done_callback",
        (pyrt_generic::PyDoneCallback {
            cancel_tx: Some(cancel_tx),
        },),
    )?;

    let future_tx1 = PyObject::from(py_fut);
    let future_tx2 = future_tx1.clone();
    let rth = rt.handler();

    rt.spawn_local(async move {
        let rti = rth.handler();
        let locals2 = locals.clone();

        if let Err(e) = rth.spawn_local(async move {
            let result = rti.scope_local(
                locals2.clone(),
                pyrt_generic::Cancellable::new_with_cancel_rx(fut, cancel_rx),
            )
            .await;

            Python::with_gil(move |py| {
                if pyrt_generic::cancelled(future_tx1.as_ref(py))
                    .map_err(pyo3_asyncio::dump_err(py))
                    .unwrap_or(false)
                {
                    return;
                }

                let _ = pyrt_generic::set_result(
                    locals2.event_loop(py),
                    future_tx1.as_ref(py),
                    result.map(|val| val.into_py(py)),
                )
                .map_err(pyo3_asyncio::dump_err(py));
            });
        })
        .await
        {
            if e.is_panic() {
                Python::with_gil(move |py| {
                    if pyrt_generic::cancelled(future_tx2.as_ref(py))
                        .map_err(pyo3_asyncio::dump_err(py))
                        .unwrap_or(false)
                    {
                        return;
                    }

                    let _ = pyrt_generic::set_result(
                        locals.event_loop(py),
                        future_tx2.as_ref(py),
                        Err(pyo3_asyncio::err::RustPanic::new_err("Rust future panicked")),
                    )
                    .map_err(pyo3_asyncio::dump_err(py));
                });
            }
        }
    });

    Ok(py_fut)
}

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

pub(crate) fn future_into_py<R, F, T>(rt: R, py: Python, fut: F) -> PyResult<&PyAny>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPy<PyObject>,
{
    future_into_py_with_locals::<R, F, T>(rt, py, get_current_locals::<R>(py)?, fut)
}

#[allow(dead_code)]
pub fn local_future_into_py<R, F, T>(rt: R, py: Python, fut: F) -> PyResult<&PyAny>
where
    R: Runtime + ContextExt + SpawnLocalExt + LocalContextExt,
    F: Future<Output = PyResult<T>> + 'static,
    T: IntoPy<PyObject>,
{
    local_future_into_py_with_locals::<R, F, T>(rt, py, get_current_locals::<R>(py)?, fut)
}

pub(crate) fn run_until_complete<R, F, T>(rt: R, event_loop: &PyAny, fut: F) -> PyResult<T>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: Send + Sync + 'static,
{
    let py = event_loop.py();
    let result_tx = Arc::new(Mutex::new(None));
    let result_rx = Arc::clone(&result_tx);
    let coro = future_into_py_with_locals::<R, _, ()>(
        rt,
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

pub(crate) fn block_on_local<F>(rt: RuntimeWrapper, local: LocalSet, fut: F)
where
    F: Future + 'static
{
    local.block_on(&rt.rt, fut);
}
