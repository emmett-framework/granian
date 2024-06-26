use pyo3::{exceptions::PyStopIteration, prelude::*};

use std::sync::{atomic, Arc, RwLock};
use tokio::sync::Notify;

use super::asyncio::PyContext;

#[derive(Clone)]
pub(crate) struct CallbackWrapper {
    pub callback: Arc<PyObject>,
    pub context: PyContext,
}

impl CallbackWrapper {
    pub(crate) fn new(callback: PyObject, event_loop: Bound<PyAny>, context: Bound<PyAny>) -> Self {
        Self {
            callback: Arc::new(callback),
            context: PyContext::new(event_loop).with_context(context),
        }
    }
}

#[pyclass(frozen)]
pub(crate) struct PyEmptyAwaitable;

#[pymethods]
impl PyEmptyAwaitable {
    fn __await__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(&self) -> Option<()> {
        None
    }
}

#[pyclass(frozen)]
pub(crate) struct PyIterAwaitable {
    result: RwLock<Option<PyResult<PyObject>>>,
}

#[cfg(not(target_os = "linux"))]
impl PyIterAwaitable {
    pub(crate) fn new() -> Self {
        Self {
            result: RwLock::new(None),
        }
    }

    pub(crate) fn set_result(&self, result: PyResult<impl IntoPy<PyObject>>) {
        let mut res = self.result.write().unwrap();
        *res = Some(Python::with_gil(|py| result.map(|v| v.into_py(py))));
    }
}

#[pymethods]
impl PyIterAwaitable {
    fn __await__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(&self, py: Python) -> PyResult<Option<PyObject>> {
        if let Ok(res) = self.result.try_read() {
            if let Some(ref res) = *res {
                return res
                    .as_ref()
                    .map_err(|err| err.clone_ref(py))
                    .map(|v| Err(PyStopIteration::new_err(v.clone_ref(py))))?;
            }
        };
        Ok(Some(py.None()))
    }
}

enum PyFutureAwaitableState {
    Pending,
    Completed(PyResult<PyObject>),
    Cancelled,
}

#[pyclass(frozen)]
pub(crate) struct PyFutureAwaitable {
    state: RwLock<PyFutureAwaitableState>,
    event_loop: PyObject,
    cancel_tx: Arc<Notify>,
    py_block: atomic::AtomicBool,
    ack: RwLock<Option<(PyObject, Py<pyo3::types::PyDict>)>>,
}

impl PyFutureAwaitable {
    pub(crate) fn new(event_loop: PyObject) -> Self {
        Self {
            state: RwLock::new(PyFutureAwaitableState::Pending),
            event_loop,
            cancel_tx: Arc::new(Notify::new()),
            py_block: true.into(),
            ack: RwLock::new(None),
        }
    }

    pub fn to_spawn(self, py: Python) -> PyResult<(Py<PyFutureAwaitable>, Arc<Notify>)> {
        let cancel_tx = self.cancel_tx.clone();
        Ok((Py::new(py, self)?, cancel_tx))
    }

    pub(crate) fn set_result(&self, result: PyResult<impl IntoPy<PyObject>>, aw: Py<PyFutureAwaitable>) {
        Python::with_gil(|py| {
            let mut state = self.state.write().unwrap();
            if !matches!(&mut *state, PyFutureAwaitableState::Pending) {
                return;
            }
            *state = PyFutureAwaitableState::Completed(result.map(|v| v.into_py(py)));

            let ack = self.ack.read().unwrap();
            if let Some((cb, ctx)) = &*ack {
                let _ = self.event_loop.clone_ref(py).call_method_bound(
                    py,
                    pyo3::intern!(py, "call_soon_threadsafe"),
                    (cb, aw),
                    Some(ctx.bind(py)),
                );
            }
        });
    }
}

#[pymethods]
impl PyFutureAwaitable {
    fn __await__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(pyself: PyRef<'_, Self>) -> PyResult<Option<PyRef<'_, Self>>> {
        let state = pyself.state.read().unwrap();
        if let PyFutureAwaitableState::Completed(res) = &*state {
            let py = pyself.py();
            return res
                .as_ref()
                .map_err(|err| err.clone_ref(py))
                .map(|v| Err(PyStopIteration::new_err(v.clone_ref(py))))?;
        };
        drop(state);
        Ok(Some(pyself))
    }

    #[getter(_asyncio_future_blocking)]
    fn get_block(&self) -> bool {
        self.py_block.load(atomic::Ordering::Relaxed)
    }

    #[setter(_asyncio_future_blocking)]
    fn set_block(&self, val: bool) {
        self.py_block.store(val, atomic::Ordering::Relaxed);
    }

    fn get_loop(&self, py: Python) -> PyObject {
        self.event_loop.clone_ref(py)
    }

    #[pyo3(signature = (cb, context=None))]
    fn add_done_callback(pyself: PyRef<'_, Self>, cb: PyObject, context: Option<PyObject>) -> PyResult<()> {
        let py = pyself.py();
        let kwctx = pyo3::types::PyDict::new_bound(py);
        kwctx.set_item(pyo3::intern!(py, "context"), context)?;

        let state = pyself.state.read().unwrap();
        match &*state {
            PyFutureAwaitableState::Pending => {
                let mut ack = pyself.ack.write().unwrap();
                *ack = Some((cb, kwctx.unbind()));
                Ok(())
            }
            _ => {
                drop(state);
                let event_loop = pyself.event_loop.clone_ref(py);
                event_loop.call_method_bound(py, pyo3::intern!(py, "call_soon"), (cb, pyself), Some(&kwctx))?;
                Ok(())
            }
        }
    }

    #[allow(unused)]
    fn remove_done_callback(&self, cb: PyObject) -> i32 {
        let mut ack = self.ack.write().unwrap();
        *ack = None;
        1
    }

    #[allow(unused)]
    #[pyo3(signature = (msg=None))]
    fn cancel(pyself: PyRef<'_, Self>, msg: Option<PyObject>) -> bool {
        let mut state = pyself.state.write().unwrap();
        if !matches!(&mut *state, PyFutureAwaitableState::Pending) {
            return false;
        }

        pyself.cancel_tx.notify_one();
        *state = PyFutureAwaitableState::Cancelled;

        let ack = pyself.ack.read().unwrap();
        if let Some((cb, ctx)) = &*ack {
            let py = pyself.py();
            let event_loop = pyself.event_loop.clone_ref(py);
            let cb = cb.clone_ref(py);
            let ctx = ctx.clone_ref(py);
            drop(ack);
            drop(state);

            let _ = event_loop.call_method_bound(py, pyo3::intern!(py, "call_soon"), (cb, pyself), Some(ctx.bind(py)));
        }

        true
    }

    fn done(&self) -> bool {
        let state = self.state.read().unwrap();
        !matches!(&*state, PyFutureAwaitableState::Pending)
    }

    fn result(&self, py: Python) -> PyResult<PyObject> {
        let state = self.state.read().unwrap();
        match &*state {
            PyFutureAwaitableState::Completed(res) => {
                res.as_ref().map(|v| v.clone_ref(py)).map_err(|err| err.clone_ref(py))
            }
            PyFutureAwaitableState::Cancelled => {
                Err(pyo3::exceptions::asyncio::CancelledError::new_err("Future cancelled."))
            }
            PyFutureAwaitableState::Pending => Err(pyo3::exceptions::asyncio::InvalidStateError::new_err(
                "Result is not ready.",
            )),
        }
    }

    fn exception(&self, py: Python) -> PyResult<PyObject> {
        let state = self.state.read().unwrap();
        match &*state {
            PyFutureAwaitableState::Completed(res) => res.as_ref().map(|_| py.None()).map_err(|err| err.clone_ref(py)),
            PyFutureAwaitableState::Cancelled => {
                Err(pyo3::exceptions::asyncio::CancelledError::new_err("Future cancelled."))
            }
            PyFutureAwaitableState::Pending => Err(pyo3::exceptions::asyncio::InvalidStateError::new_err(
                "Exception is not set.",
            )),
        }
    }
}

#[pyclass(frozen)]
pub(crate) struct PyFutureDoneCallback {
    pub cancel_tx: Arc<Notify>,
}

#[pymethods]
impl PyFutureDoneCallback {
    pub fn __call__(&self, fut: Bound<PyAny>) -> PyResult<()> {
        let py = fut.py();

        if { fut.getattr(pyo3::intern!(py, "cancelled"))?.call0()?.is_truthy() }.unwrap_or(false) {
            self.cancel_tx.notify_one();
        }

        Ok(())
    }
}

#[pyclass(frozen)]
pub(crate) struct PyFutureResultSetter;

#[pymethods]
impl PyFutureResultSetter {
    pub fn __call__(&self, target: Bound<PyAny>, value: Bound<PyAny>) {
        let _ = target.call1((value,));
    }
}

macro_rules! callback_impl_run {
    () => {
        pub fn run(self, py: Python<'_>) -> PyResult<Bound<PyAny>> {
            let event_loop = self.context.event_loop(py);
            let target = self.into_py(py).getattr(py, pyo3::intern!(py, "_loop_task"))?;
            let kwctx = pyo3::types::PyDict::new_bound(py);
            kwctx.set_item(pyo3::intern!(py, "context"), crate::asyncio::empty_context(py)?)?;
            event_loop.call_method(pyo3::intern!(py, "call_soon_threadsafe"), (target,), Some(&kwctx))
        }
    };
}

macro_rules! callback_impl_run_pytask {
    () => {
        pub fn run(self, py: Python<'_>) -> PyResult<Bound<PyAny>> {
            let event_loop = self.context.event_loop(py);
            let context = self.context.context(py);
            let target = self.into_py(py).getattr(py, pyo3::intern!(py, "_loop_task"))?;
            let kwctx = pyo3::types::PyDict::new_bound(py);
            kwctx.set_item(pyo3::intern!(py, "context"), context)?;
            event_loop.call_method(pyo3::intern!(py, "call_soon_threadsafe"), (target,), Some(&kwctx))
        }
    };
}

macro_rules! callback_impl_loop_run {
    () => {
        pub fn run(self, py: Python<'_>) -> PyResult<Bound<PyAny>> {
            let context = self.pycontext.clone_ref(py).into_bound(py);
            context.call_method1(
                pyo3::intern!(py, "run"),
                (self.into_py(py).getattr(py, pyo3::intern!(py, "_loop_step"))?,),
            )
        }
    };
}

macro_rules! callback_impl_loop_pytask {
    ($pyself:expr, $py:expr) => {
        $pyself.context.event_loop($py).call_method1(
            pyo3::intern!($py, "create_task"),
            ($pyself
                .cb
                .clone_ref($py)
                .into_bound($py)
                .call1(($pyself.into_py($py),))?,),
        )
    };
}

macro_rules! callback_impl_loop_step {
    ($pyself:expr, $py:expr) => {
        match $pyself.cb.call_method1($py, pyo3::intern!($py, "send"), ($py.None(),)) {
            Ok(res) => {
                let blocking: bool = match res.getattr($py, pyo3::intern!($py, "_asyncio_future_blocking")) {
                    Ok(v) => v.extract($py)?,
                    _ => false,
                };

                let ctx = $pyself.pycontext.clone_ref($py);
                let kwctx = pyo3::types::PyDict::new_bound($py);
                kwctx.set_item(pyo3::intern!($py, "context"), ctx)?;

                match blocking {
                    true => {
                        res.setattr($py, pyo3::intern!($py, "_asyncio_future_blocking"), false)?;
                        res.call_method_bound(
                            $py,
                            pyo3::intern!($py, "add_done_callback"),
                            ($pyself.into_py($py).getattr($py, pyo3::intern!($py, "_loop_wake"))?,),
                            Some(&kwctx),
                        )?;
                        Ok(())
                    }
                    false => {
                        let event_loop = $pyself.context.event_loop($py);
                        event_loop.call_method(
                            pyo3::intern!($py, "call_soon"),
                            ($pyself.into_py($py).getattr($py, pyo3::intern!($py, "_loop_step"))?,),
                            Some(&kwctx),
                        )?;
                        Ok(())
                    }
                }
            }
            Err(err) => {
                if (err.is_instance_of::<pyo3::exceptions::PyStopIteration>($py)
                    || err.is_instance_of::<pyo3::exceptions::asyncio::CancelledError>($py))
                {
                    $pyself.done();
                } else {
                    $pyself.err(&err);
                }
                Ok(())
            }
        }
    };
}

macro_rules! callback_impl_loop_wake {
    ($pyself:expr, $py:expr, $fut:expr) => {
        match $fut.call_method0($py, pyo3::intern!($py, "result")) {
            Ok(_) => $pyself.into_py($py).call_method0($py, pyo3::intern!($py, "_loop_step")),
            Err(err) => $pyself._loop_err($py, err),
        }
    };
}

macro_rules! callback_impl_loop_err {
    () => {
        pub fn _loop_err(&self, py: Python, err: PyErr) -> PyResult<PyObject> {
            self.err(&err);
            let cberr = self.cb.call_method1(py, pyo3::intern!(py, "throw"), (err,));
            cberr
        }
    };
}

pub(crate) use callback_impl_loop_err;
pub(crate) use callback_impl_loop_pytask;
pub(crate) use callback_impl_loop_run;
pub(crate) use callback_impl_loop_step;
pub(crate) use callback_impl_loop_wake;
pub(crate) use callback_impl_run;
pub(crate) use callback_impl_run_pytask;
