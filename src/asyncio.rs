use pyo3::{prelude::*, sync::GILOnceCell};
use std::convert::Into;

static ASYNCIO: GILOnceCell<PyObject> = GILOnceCell::new();
static ASYNCIO_LOOP: GILOnceCell<PyObject> = GILOnceCell::new();
static CONTEXTVARS: GILOnceCell<PyObject> = GILOnceCell::new();

fn asyncio(py: Python) -> PyResult<&PyAny> {
    ASYNCIO
        .get_or_try_init(py, || Ok(py.import("asyncio")?.into()))
        .map(|asyncio| asyncio.as_ref(py))
}

pub(crate) fn get_running_loop(py: Python) -> PyResult<&PyAny> {
    ASYNCIO_LOOP
        .get_or_try_init(py, || -> PyResult<PyObject> {
            let asyncio = asyncio(py)?;

            Ok(asyncio.getattr("get_running_loop")?.into())
        })?
        .as_ref(py)
        .call0()
}

fn contextvars(py: Python) -> PyResult<&PyAny> {
    Ok(CONTEXTVARS
        .get_or_try_init(py, || py.import("contextvars").map(Into::into))?
        .as_ref(py))
}

pub(crate) fn copy_context(py: Python) -> PyResult<&PyAny> {
    contextvars(py)?.call_method0("copy_context")
}
