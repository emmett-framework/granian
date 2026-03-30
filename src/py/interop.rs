use pyo3::prelude::*;
use std::sync::Arc;

use crate::utils::log_application_callable_exception;

pub(crate) type ArcApp = Arc<Py<PyApp>>;

#[pyclass(frozen, module = "granian._granian", name = "AbortHandle")]
pub(crate) struct PyAbortHandle {
    handle: tokio::task::AbortHandle,
}

impl PyAbortHandle {
    pub fn new(handle: tokio::task::AbortHandle) -> Self {
        Self { handle }
    }
}

#[pymethods]
impl PyAbortHandle {
    fn __call__(&self) {
        self.handle.abort();
    }
}

#[pyclass(frozen, module = "granian._granian", name = "App")]
pub(crate) struct PyApp {
    on_request: Py<PyAny>,
    on_websocket: Py<PyAny>,
}

impl PyApp {
    #[inline]
    pub(crate) fn handle_request<'py, A: pyo3::call::PyCallArgs<'py>>(&self, py: Python<'py>, args: A) {
        if let Err(err) = self.on_request.bind(py).call1(args) {
            log_application_callable_exception(py, &err);
        }
    }

    #[inline]
    pub(crate) fn handle_websocket<'py, A: pyo3::call::PyCallArgs<'py>>(&self, py: Python<'py>, args: A) {
        if let Err(err) = self.on_websocket.bind(py).call1(args) {
            log_application_callable_exception(py, &err);
        }
    }
}

#[pymethods]
impl PyApp {
    #[new]
    fn new(on_request: Py<PyAny>, on_websocket: Py<PyAny>) -> Self {
        Self {
            on_request,
            on_websocket,
        }
    }
}
