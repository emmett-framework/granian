use pyo3::prelude::*;
use std::collections::HashMap;
use tokio::sync::oneshot;

use crate::callbacks::CallbackWrapper;
use super::{
    errors::ApplicationError,
    io::{RSGIHTTPProtocol as HTTPProtocol, RSGIWebsocketProtocol as WebsocketProtocol},
    types::RSGIScope as Scope
};


#[derive(FromPyObject)]
pub(crate) struct CallbackResponse {
    pub mode: u32,
    pub status: i32,
    pub headers: HashMap<String, String>,
    pub bytes_data: Option<Vec<u8>>,
    pub str_data: Option<String>,
    pub file_path: Option<String>
}

#[pyclass]
pub(crate) struct CallbackResponseWatcher {
    tx: Option<oneshot::Sender<CallbackResponse>>,
    #[pyo3(get)]
    event_loop: PyObject,
    #[pyo3(get)]
    context: PyObject
}

impl CallbackResponseWatcher {
    pub fn new(
        py: Python,
        cb: CallbackWrapper,
        tx: oneshot::Sender<CallbackResponse>
    ) -> Self {
        Self {
            tx: Some(tx),
            event_loop: cb.context.event_loop(py).into(),
            context: cb.context.context(py).into(),
        }
    }
}

#[pymethods]
impl CallbackResponseWatcher {
    fn done(&mut self, py: Python, result: PyObject) -> PyResult<()> {
        if let Some(tx) = self.tx.take() {
            // FIXME: handle failure
            let _ = tx.send(result.extract(py)?);
        };
        Ok(())
    }
}

#[pyclass]
pub(crate) struct CallbackProtocolWatcher {
    tx: Option<oneshot::Sender<(i32, bool)>>,
    #[pyo3(get)]
    event_loop: PyObject,
    #[pyo3(get)]
    context: PyObject
}

impl CallbackProtocolWatcher {
    pub fn new(
        py: Python,
        cb: CallbackWrapper,
        tx: oneshot::Sender<(i32, bool)>
    ) -> Self {
        Self {
            tx: Some(tx),
            event_loop: cb.context.event_loop(py).into(),
            context: cb.context.context(py).into(),
        }
    }
}

#[pymethods]
impl CallbackProtocolWatcher {
    fn done(&mut self, py: Python, result: PyObject) -> PyResult<()> {
        if let Some(tx) = self.tx.take() {
            // FIXME: handle failure
            let _ = tx.send(result.extract(py)?);
        };
        Ok(())
    }
}

pub(crate) async fn call_response(
    cb: CallbackWrapper,
    protocol: HTTPProtocol,
    scope: Scope
) -> PyResult<CallbackResponse> {
    let (tx, rx) = oneshot::channel();
    let callback = cb.callback.clone();
    Python::with_gil(|py| {
        callback.call1(py, (CallbackResponseWatcher::new(py, cb, tx), scope, protocol))
    })?;

    match rx.await {
        Ok(v) => Ok(v),
        _ => Err(ApplicationError.into())
    }
}

pub(crate) async fn call_protocol(
    cb: CallbackWrapper,
    protocol: WebsocketProtocol,
    scope: Scope
) -> PyResult<(i32, bool)> {
    let (tx, rx) = oneshot::channel();
    let callback = cb.callback.clone();
    Python::with_gil(|py| {
        callback.call1(py, (CallbackProtocolWatcher::new(py, cb, tx), scope, protocol))
    })?;

    match rx.await {
        Ok(v) => Ok(v),
        _ => Err(ApplicationError.into())
    }
}
