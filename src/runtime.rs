use once_cell::{sync::OnceCell, unsync::OnceCell as UnsyncOnceCell};
use pyo3_asyncio::{TaskLocals, generic as pyrt_generic};
use pyo3::prelude::*;
use std::{future::Future, io, pin::Pin};
use tokio::{runtime::Builder, task::{JoinError, JoinHandle, LocalSet}};


thread_local!(
    // static RUNTIME: OnceCell<tokio::runtime::Handle> = OnceCell::new();
    static RUNTIME: OnceCell<tokio::runtime::Runtime> = OnceCell::new();
    // static RUNTIME: RefCell<Option<tokio::runtime::Runtime>> = RefCell::new(None);
);

tokio::task_local! {
    static TASK_LOCALS: UnsyncOnceCell<TaskLocals>;
}

pub(crate) enum ThreadIsolation {
    Runtime,
    Worker
}

struct PyRuntime;

impl pyrt_generic::Runtime for PyRuntime {
    type JoinError = JoinError;
    type JoinHandle = JoinHandle<()>;

    fn spawn<F>(fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        // current_runtime().spawn(async move {
        //     fut.await;
        // })
        RUNTIME.with(|cell| {
            cell.get().unwrap().spawn(async move {
                fut.await;
            })
        })
    }
}

impl pyrt_generic::ContextExt for PyRuntime {
    fn scope<F, R>(locals: TaskLocals, fut: F) -> Pin<Box<dyn Future<Output = R> + Send>>
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

fn default_runtime() -> io::Result<tokio::runtime::Runtime> {
    Builder::new_current_thread()
        .enable_all()
        .build()
}

pub(crate) fn init_runtime() -> io::Result<tokio::runtime::Handle> {
    let rt = default_runtime()?;
    let handle = rt.handle().to_owned();
    RUNTIME.with(|cell| {
        // cell.set(rt.handle().to_owned());
        // cell.set(rt).and_then(|| { Ok(handle) })
        match cell.set(rt) {
            Ok(_) => Ok(handle),
            _ => panic!("Runtime already initialized")
        }
        // *cell.borrow_mut() = Some(rt);
        // Ok(handle)
    })
    // Ok(handle)
}

pub(crate) fn current_runtime() -> tokio::runtime::Handle {
    RUNTIME.with(|cell| match cell.get() {
        Some(rt) => rt.handle().to_owned(),
        // Some(rt) => rt,
        None => panic!("Runtime not initialized")
    })
    // RUNTIME.with(|cell| {
    //     (*cell.borrow()).unwrap()
    // })
    // RUNTIME.with(|cell| match cell.get() {
    //     Some(ref rt) => *rt,
    //     None => panic!("Runtime not initialized")
    // })
}

pub(crate) fn run_until_complete<F, T>(event_loop: &PyAny, fut: F) -> PyResult<T>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: Send + Sync + 'static,
{
    pyrt_generic::run_until_complete::<PyRuntime, _, T>(event_loop, fut)
}

pub(crate) fn future_into_py<F, T>(py: Python, fut: F) -> PyResult<&PyAny>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPy<PyObject>,
{
    pyrt_generic::future_into_py::<PyRuntime, _, T>(py, fut)
}

pub(crate) fn block_on_local<F>(local: LocalSet, fut: F)
where
    F: Future + 'static
{
    RUNTIME.with(|cell| {
        local.block_on(cell.get().unwrap(), fut)
    });
}
