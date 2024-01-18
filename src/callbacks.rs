use pyo3::{prelude::*, pyclass::IterNextOutput, sync::GILOnceCell};
use std::sync::Arc;
use tokio::sync::Notify;

static CONTEXTVARS: GILOnceCell<PyObject> = GILOnceCell::new();
static CONTEXT: GILOnceCell<PyObject> = GILOnceCell::new();

#[derive(Clone)]
pub(crate) struct CallbackWrapper {
    pub callback: PyObject,
    pub context: pyo3_asyncio::TaskLocals,
}

impl CallbackWrapper {
    pub(crate) fn new(callback: PyObject, event_loop: &PyAny, context: &PyAny) -> Self {
        Self {
            callback,
            context: pyo3_asyncio::TaskLocals::new(event_loop).with_context(context),
        }
    }
}

#[pyclass]
pub(crate) struct PyEmptyAwaitable {}

#[pymethods]
impl PyEmptyAwaitable {
    fn __await__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(&self, py: Python) -> IterNextOutput<PyObject, PyObject> {
        IterNextOutput::Return(py.None())
    }
}

#[pyclass]
pub(crate) struct PyIterAwaitable {
    result: Option<PyResult<PyObject>>,
}

impl PyIterAwaitable {
    pub(crate) fn new() -> Self {
        Self { result: None }
    }

    pub(crate) fn set_result(&mut self, result: PyResult<PyObject>) {
        self.result = Some(result);
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

    fn __next__(&self, py: Python) -> PyResult<IterNextOutput<PyObject, PyObject>> {
        match &self.result {
            Some(res) => match res {
                Ok(v) => Ok(IterNextOutput::Return(v.clone_ref(py))),
                Err(err) => Err(err.clone_ref(py)),
            },
            _ => Ok(IterNextOutput::Yield(py.None())),
        }
    }
}

#[pyclass]
pub(crate) struct PyFutureAwaitable {
    fut_spawner: Option<Box<dyn FnOnce(Option<PyObject>, Arc<Notify>, Py<PyFutureAwaitable>) + Send>>,
    result: Option<PyResult<PyObject>>,
    event_loop: PyObject,
    callback: Option<PyObject>,
    cancel_tx: Arc<Notify>,
    py_block: bool,
    py_cancelled: bool,
}

impl PyFutureAwaitable {
    pub(crate) fn new(
        fut_spawner: Box<dyn FnOnce(Option<PyObject>, Arc<Notify>, Py<PyFutureAwaitable>) + Send>,
        event_loop: PyObject,
    ) -> Self {
        Self {
            fut_spawner: Some(fut_spawner),
            result: None,
            event_loop,
            callback: None,
            cancel_tx: Arc::new(Notify::new()),
            py_block: true,
            py_cancelled: false,
        }
    }

    pub(crate) fn set_result(mut pyself: PyRefMut<'_, Self>, result: PyResult<PyObject>) -> Option<PyObject> {
        pyself.result = Some(result);
        pyself.callback.take()
    }
}

#[pymethods]
impl PyFutureAwaitable {
    fn __await__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    #[getter(_asyncio_future_blocking)]
    fn get_block(&self) -> bool {
        self.py_block
    }

    #[setter(_asyncio_future_blocking)]
    fn set_block(&mut self, val: bool) {
        self.py_block = val;
    }

    fn get_loop(&self, py: Python) -> PyObject {
        self.event_loop.clone_ref(py)
    }

    #[pyo3(signature = (cb, context=None))]
    fn add_done_callback(mut pyself: PyRefMut<'_, Self>, cb: PyObject, context: Option<PyObject>) -> PyResult<()> {
        pyself.callback = Some(cb);
        if let Some(spawner) = pyself.fut_spawner.take() {
            (spawner)(context, pyself.cancel_tx.clone(), pyself.into());
        }
        Ok(())
    }

    #[allow(unused)]
    fn remove_done_callback(&mut self, cb: PyObject) -> i32 {
        self.callback = None;
        1
    }

    #[allow(unused)]
    #[pyo3(signature = (msg=None))]
    fn cancel(&mut self, msg: Option<PyObject>) -> bool {
        if self.done() {
            return false;
        }
        self.py_cancelled = true;
        self.cancel_tx.notify_one();
        true
    }

    fn done(&self) -> bool {
        self.result.is_some() || self.py_cancelled
    }

    fn result(pyself: PyRef<'_, Self>) -> PyResult<PyObject> {
        match &pyself.result {
            Some(res) => {
                let py = pyself.py();
                res.as_ref().map(|v| v.clone_ref(py)).map_err(|err| err.clone_ref(py))
            }
            _ => Ok(pyself.py().None()),
        }
    }

    fn exception(&self) {}

    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(pyself: PyRef<'_, Self>) -> PyResult<IterNextOutput<PyRef<'_, Self>, PyObject>> {
        match &pyself.result {
            Some(res) => {
                let py = pyself.py();
                res.as_ref()
                    .map(|v| IterNextOutput::Return(v.clone_ref(py)))
                    .map_err(|err| err.clone_ref(py))
            }
            _ => Ok(IterNextOutput::Yield(pyself)),
        }
    }
}

fn contextvars(py: Python) -> PyResult<&PyAny> {
    Ok(CONTEXTVARS
        .get_or_try_init(py, || py.import("contextvars").map(std::convert::Into::into))?
        .as_ref(py))
}

pub fn empty_pycontext(py: Python) -> PyResult<&PyAny> {
    Ok(CONTEXT
        .get_or_try_init(py, || {
            contextvars(py)?
                .getattr("Context")?
                .call0()
                .map(std::convert::Into::into)
        })?
        .as_ref(py))
}

macro_rules! callback_impl_run {
    () => {
        pub fn run(self, py: Python<'_>) -> PyResult<&PyAny> {
            let event_loop = self.context.event_loop(py);
            let target = self.into_py(py).getattr(py, pyo3::intern!(py, "_loop_task"))?;
            let kwctx = pyo3::types::PyDict::new(py);
            kwctx.set_item(
                pyo3::intern!(py, "context"),
                crate::callbacks::empty_pycontext(py)?,
            )?;
            event_loop.call_method(pyo3::intern!(py, "call_soon_threadsafe"), (target,), Some(kwctx))
        }
    };
}

macro_rules! callback_impl_run_pytask {
    () => {
        pub fn run(self, py: Python<'_>) -> PyResult<&PyAny> {
            let event_loop = self.context.event_loop(py);
            let context = self.context.context(py);
            let target = self.into_py(py).getattr(py, pyo3::intern!(py, "_loop_task"))?;
            let kwctx = pyo3::types::PyDict::new(py);
            kwctx.set_item(pyo3::intern!(py, "context"), context)?;
            event_loop.call_method(pyo3::intern!(py, "call_soon_threadsafe"), (target,), Some(kwctx))
        }
    };
}

macro_rules! callback_impl_loop_run {
    () => {
        pub fn run(self, py: Python<'_>) -> PyResult<&PyAny> {
            let context = self.pycontext.clone_ref(py).into_ref(py);
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
            ($pyself.cb.clone_ref($py).into_ref($py).call1(($pyself.into_py($py),))?,),
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
                let kwctx = pyo3::types::PyDict::new($py);
                kwctx.set_item(pyo3::intern!($py, "context"), ctx)?;

                match blocking {
                    true => {
                        res.setattr($py, pyo3::intern!($py, "_asyncio_future_blocking"), false)?;
                        res.call_method(
                            $py,
                            pyo3::intern!($py, "add_done_callback"),
                            ($pyself.into_py($py).getattr($py, pyo3::intern!($py, "_loop_wake"))?,),
                            Some(kwctx),
                        )?;
                        Ok(())
                    }
                    false => {
                        let event_loop = $pyself.context.event_loop($py);
                        event_loop.call_method(
                            pyo3::intern!($py, "call_soon"),
                            ($pyself.into_py($py).getattr($py, pyo3::intern!($py, "_loop_step"))?,),
                            Some(kwctx),
                        )?;
                        Ok(())
                    }
                }
            }
            Err(err) => {
                if (err.is_instance_of::<pyo3::exceptions::PyStopIteration>($py)
                    || err.is_instance_of::<pyo3::exceptions::asyncio::CancelledError>($py))
                {
                    $pyself.done($py);
                    Ok(())
                } else {
                    $pyself.err($py);
                    Err(err)
                }
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
            let cberr = self.cb.call_method1(py, pyo3::intern!(py, "throw"), (err,));
            self.err(py);
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
