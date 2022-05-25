use hyper::{Response, Body};
use pyo3::prelude::*;
use tokio::sync::oneshot;

use super::super::{callbacks::CallbackWrapper, io::Receiver};
use super::io::Sender;
use super::types::Scope;

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
        tx: Option<oneshot::Sender<bool>>
    ) -> Self {
        Self {
            tx: tx,
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

// pub(crate) async fn acall(
//     cb: CallbackWrapper,
//     receiver: Receiver,
//     sender: Sender,
//     scope: Scope
// ) -> PyResult<()> {
//     Python::with_gil(|py| {
//         let coro = cb.callback.call1(py, (scope, receiver, sender))?;
//         pyo3_asyncio::into_future_with_locals(
//             &cb.context,
//             coro.as_ref(py)
//         )
//     })?
//     .await?;
//     Ok(())
// }

pub(crate) async fn call(
    cb: CallbackWrapper,
    receiver: Receiver,
    scope: Scope
) -> Result<oneshot::Receiver<Response<Body>>, super::errors::ASGIFlowError> {
    let (tx, rx) = oneshot::channel();
    let (stx, srx) = oneshot::channel();

    let callback = cb.callback.clone();
    let sender = Sender::new(Some(stx));
    Python::with_gil(|py| {
        callback.call1(
            py,
            (CallbackWatcher::new(py, cb, Some(tx)), scope, receiver, sender)
        )
    })?;

    match rx.await {
        Ok(true) => Ok(srx),
        _ => Err(super::errors::ASGIFlowError)
    }
}
