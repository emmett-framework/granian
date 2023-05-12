use pyo3::prelude::*;
use pyo3::pyclass::IterNextOutput;


#[derive(Clone)]
pub(crate) struct CallbackWrapper {
    pub callback: PyObject,
    pub context: pyo3_asyncio::TaskLocals
}

impl CallbackWrapper {
    pub(crate) fn new(
        callback: PyObject,
        event_loop: &PyAny,
        context: &PyAny
    ) -> Self {
        Self {
            callback,
            context: pyo3_asyncio::TaskLocals::new(event_loop).with_context(context)
        }
    }
}

#[pyclass]
pub(crate) struct PyAwaitableResultYielder {
    result: Option<PyResult<PyObject>>,
    none: PyObject
}

impl PyAwaitableResultYielder {
    pub(crate) fn set_result(&mut self, result: PyResult<PyObject>) {
        self.result = Some(result)
    }
}

#[pymethods]
impl PyAwaitableResultYielder {
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(mut pyself: PyRefMut<'_, Self>) -> PyResult<IterNextOutput<PyObject, PyObject>> {
        match pyself.result.take() {
            Some(res) => {
                match res {
                    Ok(v) => Ok(IterNextOutput::Return(v)),
                    Err(err) => Err(err)
                }
            },
            _ => Ok(IterNextOutput::Yield(pyself.none.clone()))
        }
    }
}

#[pyclass]
pub(crate) struct PyAwaitableResultFutureLike {
    py_block: bool,
    event_loop: PyObject,
    result: Option<PyResult<PyObject>>,
    cb: Option<(PyObject, Py<pyo3::types::PyDict>)>
}

impl PyAwaitableResultFutureLike {
    pub(crate) fn new(event_loop: PyObject) -> Self {
        Self {
            event_loop,
            py_block: true,
            result: None,
            cb: None
        }
    }

    pub(crate) fn set_result(mut pyself: PyRefMut<'_, Self>, result: PyResult<PyObject>) {
        pyself.result = Some(result);
        if let Some((cb, ctx)) = pyself.cb.take() {
            Python::with_gil(|py| {
                let _ = pyself.event_loop.call_method(
                    py,
                    "call_soon_threadsafe",
                    (cb, &pyself),
                    Some(ctx.as_ref(py))
                );
            })
        }
    }
}

#[pymethods]
impl PyAwaitableResultFutureLike {
    #[getter(_asyncio_future_blocking)]
    fn get_block(&self) -> bool {
        self.py_block
    }

    #[setter(_asyncio_future_blocking)]
    fn set_block(&mut self, val: bool) {
        self.py_block = val
    }

    fn get_loop(&mut self) -> PyObject {
        self.event_loop.clone()
    }

    fn add_done_callback(
        mut pyself: PyRefMut<'_, Self>,
        py: Python,
        cb: PyObject,
        context: PyObject
    ) -> PyResult<()> {
        let kwctx = pyo3::types::PyDict::new(py);
        kwctx.set_item("context", context)?;
        match pyself.result {
            Some(_) => {
                pyself.event_loop.call_method(py, "call_soon", (cb, &pyself), Some(kwctx))?;
            },
            _ => {
                pyself.cb = Some((cb, kwctx.into_py(py)));
            }
        }
        Ok(())
    }

    fn cancel(mut pyself: PyRefMut<'_, Self>, py: Python) -> bool {
        if let Some((cb, kwctx)) = pyself.cb.take() {
            let _ = pyself.event_loop.call_method(
                py, "call_soon", (cb, &pyself), Some(kwctx.as_ref(py))
            );
        }
        false
    }

    fn result(&self) {}
    fn exception(&self) {}

    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(
        mut pyself: PyRefMut<'_, Self>
    ) -> PyResult<IterNextOutput<PyRefMut<'_, Self>, PyObject>> {
        match pyself.result {
            Some(_) => {
                match pyself.result.take().unwrap() {
                    Ok(v) => Ok(IterNextOutput::Return(v)),
                    Err(err) => Err(err)
                }
            },
            _ => Ok(IterNextOutput::Yield(pyself))
        }
    }
}

#[pyclass]
pub(crate) struct PyIterAwaitableResult {
    pub inner: Py<PyAwaitableResultYielder>
}

impl PyIterAwaitableResult {
    pub(crate) fn new(py: Python) -> PyResult<Self> {
        let inner = Py::new(py, PyAwaitableResultYielder { result: None, none: py.None() })?;
        Ok(Self { inner })
    }
}

#[pymethods]
impl PyIterAwaitableResult {
    fn __await__(&mut self, py: Python) -> PyObject {
        self.inner.to_object(py)
    }
}

#[pyclass]
pub(crate) struct PyFutureAwaitableResult {
    pub inner: Py<PyAwaitableResultFutureLike>
}

impl PyFutureAwaitableResult {
    pub(crate) fn new(py: Python, event_loop: &PyAny) -> PyResult<Self> {
        let inner = Py::new(py, PyAwaitableResultFutureLike::new(event_loop.to_object(py)))?;
        Ok(Self { inner })
    }
}

#[pymethods]
impl PyFutureAwaitableResult {
    fn __await__(&mut self, py: Python) -> PyObject {
        self.inner.to_object(py)
    }
}

macro_rules! callback_impl_run {
    () => {
        pub fn run<'p>(self, py: Python<'p>) -> PyResult<&'p PyAny> {
            let event_loop = self.context.event_loop(py);
            let target = self.into_py(py).getattr(py, pyo3::intern!(py, "_loop_task"))?;
            event_loop.call_method1(pyo3::intern!(py, "call_soon_threadsafe"), (target,))
        }
    };
}

macro_rules! callback_impl_loop_run {
    () => {
        pub fn run<'p>(self, py: Python<'p>) -> PyResult<&'p PyAny> {
            let context = self.context.context(py);
            context.call_method1(
                pyo3::intern!(py, "run"),
                (self.into_py(py).getattr(py, pyo3::intern!(py, "_loop_step"))?,)
            )
        }
    };
}

macro_rules! callback_impl_loop_step {
    ($pyself:expr, $py:expr) => {
        match $pyself.cb.call_method1($py, pyo3::intern!($py, "send"), ($py.None(),)) {
            Ok(res) => {
                let blocking: bool = match res.getattr(
                    $py,
                    pyo3::intern!($py, "_asyncio_future_blocking")
                ) {
                    Ok(v) => v.extract($py)?,
                    _ => false
                };

                let ctx = $pyself.context.context($py);
                let kwctx = pyo3::types::PyDict::new($py);
                kwctx.set_item("context", ctx)?;

                match blocking {
                    true => {
                        res.setattr(
                            $py,
                            pyo3::intern!($py, "_asyncio_future_blocking"),
                            false
                        )?;
                        res.call_method(
                            $py,
                            pyo3::intern!($py, "add_done_callback"),
                            (
                                $pyself
                                .into_py($py)
                                .getattr($py, pyo3::intern!($py, "_loop_wake"))?,
                            ),
                            Some(kwctx)
                        )?;
                        Ok($py.None())
                    },
                    false => {
                        let event_loop = $pyself.context.event_loop($py);
                        event_loop.call_method(
                            pyo3::intern!($py, "call_soon"),
                            (
                                $pyself
                                .into_py($py)
                                .getattr($py, pyo3::intern!($py, "_loop_step"))?,
                            ),
                            Some(kwctx)
                        )?;
                        Ok($py.None())
                    }
                }
            },
            Err(err) => {
                match err.is_instance_of::<pyo3::exceptions::PyStopIteration>($py) {
                    true => {
                        $pyself.done($py);
                        Ok($py.None())
                    },
                    false => {
                        $pyself.err($py);
                        Err(err)
                    }
                }
            }
        }
    };
}

macro_rules! callback_impl_loop_wake {
    ($pyself:expr, $py:expr, $fut:expr) => {
        match $fut.call_method0($py, pyo3::intern!($py, "result")) {
            Ok(_) => $pyself.into_py($py).call_method0($py, pyo3::intern!($py, "_loop_step")),
            Err(err) => $pyself._loop_err($py, err)
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

pub(crate) use callback_impl_run;
pub(crate) use callback_impl_loop_run;
pub(crate) use callback_impl_loop_step;
pub(crate) use callback_impl_loop_wake;
pub(crate) use callback_impl_loop_err;
