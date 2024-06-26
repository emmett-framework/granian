use pyo3::{prelude::*, sync::GILOnceCell};
use std::{convert::Into, sync::Arc};

static CONTEXTVARS: GILOnceCell<PyObject> = GILOnceCell::new();
static CONTEXT: GILOnceCell<PyObject> = GILOnceCell::new();

#[derive(Clone, Debug)]
pub struct PyContext {
    event_loop: Arc<PyObject>,
    context: Arc<PyObject>,
}

impl PyContext {
    pub fn new(event_loop: Bound<PyAny>) -> Self {
        let pynone = event_loop.py().None();
        Self {
            event_loop: Arc::new(event_loop.unbind()),
            context: Arc::new(pynone),
        }
    }

    pub fn with_context(self, context: Bound<PyAny>) -> Self {
        Self {
            context: Arc::new(context.unbind()),
            ..self
        }
    }

    pub fn event_loop<'p>(&self, py: Python<'p>) -> Bound<'p, PyAny> {
        self.event_loop.clone_ref(py).into_bound(py)
    }

    pub fn context<'p>(&self, py: Python<'p>) -> Bound<'p, PyAny> {
        self.context.clone_ref(py).into_bound(py)
    }
}

fn contextvars(py: Python) -> PyResult<&Bound<PyAny>> {
    Ok(CONTEXTVARS
        .get_or_try_init(py, || py.import_bound("contextvars").map(Into::into))?
        .bind(py))
}

pub(crate) fn empty_context(py: Python) -> PyResult<&Bound<PyAny>> {
    Ok(CONTEXT
        .get_or_try_init(py, || {
            contextvars(py)?
                .getattr("Context")?
                .call0()
                .map(std::convert::Into::into)
        })?
        .bind(py))
}

#[allow(dead_code)]
pub(crate) fn copy_context(py: Python) -> PyResult<Bound<PyAny>> {
    contextvars(py)?.call_method0("copy_context")
}
