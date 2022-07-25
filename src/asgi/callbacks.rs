use pyo3::prelude::*;
use tokio::sync::oneshot;

use crate::callbacks::CallbackWrapper;
use super::{
    errors::{ASGIFlowError, error_flow},
    io::ASGIProtocol,
    types::ASGIScope as Scope
};


#[pyclass]
pub(crate) struct CallbackWatcher {
    tx: Option<oneshot::Sender<bool>>,
    #[pyo3(get)]
    event_loop: PyObject,
    #[pyo3(get)]
    context: PyObject
}

impl CallbackWatcher {
    pub fn new(
        py: Python,
        cb: CallbackWrapper,
        tx: oneshot::Sender<bool>
    ) -> Self {
        Self {
            tx: Some(tx),
            event_loop: cb.context.event_loop(py).into(),
            context: cb.context.context(py).into(),
        }
    }
}

#[pymethods]
impl CallbackWatcher {
    fn done(&mut self, success: bool) -> PyResult<()> {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(success);
        };
        Ok(())
    }
}

pub(crate) async fn call(
    cb: CallbackWrapper,
    protocol: impl ASGIProtocol + IntoPy<PyObject>,
    scope: Scope
) -> Result<(), ASGIFlowError> {
    let (tx, rx) = oneshot::channel();
    let callback = cb.callback.clone();
    Python::with_gil(|py| {
        callback.call1(py, (CallbackWatcher::new(py, cb, tx), scope, protocol))
    })?;

    match rx.await {
        Ok(true) => Ok(()),
        _ => error_flow!()
    }
}
