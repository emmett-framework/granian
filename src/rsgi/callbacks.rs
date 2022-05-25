use pyo3::prelude::*;
use std::collections::HashMap;
use tokio::sync::oneshot;

use super::super::{
    callbacks::CallbackWrapper,
    io::Receiver
};
use super::types::Scope;

#[derive(FromPyObject)]
pub(crate) struct CallbackRet {
    pub mode: u32,
    pub status: i32,
    pub headers: HashMap<String, String>,
    pub bytes_data: Option<Vec<u8>>,
    pub str_data: Option<String>,
    pub file_path: Option<String>
}

#[pyclass]
pub(crate) struct CallbackWatcher {
    tx: Option<oneshot::Sender<CallbackRet>>,
    #[pyo3(get)]
    event_loop: PyObject,
    #[pyo3(get)]
    context: PyObject
}

impl CallbackWatcher {
    pub fn new(
        py: Python,
        cb: CallbackWrapper,
        tx: Option<oneshot::Sender<CallbackRet>>
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
    fn done(&mut self, py: Python, result: PyObject) -> PyResult<()> {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(result.extract(py)?);
        };
        Ok(())
    }
}

// pub(crate) async fn acall(
//     cb: CallbackWrapper,
//     receiver: Receiver,
//     scope: Scope
// ) -> PyResult<CallbackRet> {
//     let res = Python::with_gil(|py| {
//         let coro = cb.callback.call1(py, (scope, receiver))?;
//         pyo3_asyncio::into_future_with_locals(
//             &cb.context,
//             coro.as_ref(py)
//         )
//     })?
//     .await?;
//     Ok(Python::with_gil(|py| { res.extract(py) })?)
// }

pub(crate) async fn call(
    cb: CallbackWrapper,
    receiver: Receiver,
    scope: Scope
) -> PyResult<CallbackRet> {
    let (tx, rx) = oneshot::channel();
    let callback = cb.callback.clone();
    Python::with_gil(|py| {
        callback.call1(py, (CallbackWatcher::new(py, cb, Some(tx)), scope, receiver))
    })?;

    match rx.await {
        Ok(v) => Ok(v),
        _ => Python::with_gil(|py| Err(PyErr::from_value(py.None().as_ref(py))))
    }
}
