use pyo3::{prelude::*, sync::GILOnceCell};
use std::convert::Into;

static ASYNCIO: GILOnceCell<PyObject> = GILOnceCell::new();
static ASYNCIO_LOOP: GILOnceCell<PyObject> = GILOnceCell::new();
static CONTEXTVARS: GILOnceCell<PyObject> = GILOnceCell::new();

fn asyncio(py: Python) -> PyResult<&Bound<PyAny>> {
    ASYNCIO
        .get_or_try_init(py, || Ok(py.import_bound("asyncio")?.into()))
        .map(|asyncio| asyncio.bind(py))
}

pub(crate) fn get_running_loop(py: Python) -> PyResult<Bound<PyAny>> {
    ASYNCIO_LOOP
        .get_or_try_init(py, || -> PyResult<PyObject> {
            let asyncio = asyncio(py)?;

            Ok(asyncio.getattr("get_running_loop")?.into())
        })?
        .bind(py)
        .call0()
}

fn contextvars(py: Python) -> PyResult<&Bound<PyAny>> {
    Ok(CONTEXTVARS
        .get_or_try_init(py, || py.import_bound("contextvars").map(Into::into))?
        .bind(py))
}

pub(crate) fn copy_context(py: Python) -> PyResult<Bound<PyAny>> {
    contextvars(py)?.call_method0("copy_context")
}
