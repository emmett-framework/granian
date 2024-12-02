use pyo3::{prelude::*, sync::GILOnceCell};
use std::convert::Into;

static CONTEXTVARS: GILOnceCell<PyObject> = GILOnceCell::new();
static CONTEXT: GILOnceCell<PyObject> = GILOnceCell::new();

fn contextvars(py: Python) -> PyResult<&Bound<PyAny>> {
    Ok(CONTEXTVARS
        .get_or_try_init(py, || py.import("contextvars").map(Into::into))?
        .bind(py))
}

#[allow(dead_code)]
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
