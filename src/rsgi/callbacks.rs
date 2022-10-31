use pyo3::prelude::*;
use tokio::sync::oneshot;

use crate::{
    callbacks::CallbackWrapper,
    runtime::RuntimeRef,
    ws::{HyperWebsocket, UpgradeData}
};
use super::{
    errors::error_proto,
    io::{RSGIHTTPProtocol as HTTPProtocol, RSGIWebsocketProtocol as WebsocketProtocol},
    types::{RSGIScope as Scope, Response}
};


#[pyclass]
pub(crate) struct CallbackWatcherHTTP {
    #[pyo3(get)]
    proto: Py<HTTPProtocol>,
    #[pyo3(get)]
    event_loop: PyObject,
    #[pyo3(get)]
    context: PyObject
}

impl CallbackWatcherHTTP {
    pub fn new(
        py: Python,
        cb: CallbackWrapper,
        proto: HTTPProtocol
    ) -> Self {
        Self {
            proto: Py::new(py, proto).unwrap(),
            event_loop: cb.context.event_loop(py).into(),
            context: cb.context.context(py).into()
        }
    }
}

#[pymethods]
impl CallbackWatcherHTTP {
    fn done(&mut self, py: Python) {
        if let Ok(mut proto) = self.proto.as_ref(py).try_borrow_mut() {
            if let (Some(tx), Some(mut res)) = proto.tx() {
                res.error();
                let _ = tx.send(res);
            }
        }
    }

    fn err(&mut self, py: Python) {
        log::warn!("Application callable raised an exception");
        self.done(py)
    }
}

#[pyclass]
pub(crate) struct CallbackWatcherWebsocket {
    #[pyo3(get)]
    proto: Py<WebsocketProtocol>,
    #[pyo3(get)]
    event_loop: PyObject,
    #[pyo3(get)]
    context: PyObject
}

impl CallbackWatcherWebsocket {
    pub fn new(
        py: Python,
        cb: CallbackWrapper,
        proto: WebsocketProtocol
    ) -> Self {
        Self {
            proto: Py::new(py, proto).unwrap(),
            event_loop: cb.context.event_loop(py).into(),
            context: cb.context.context(py).into(),
        }
    }
}

#[pymethods]
impl CallbackWatcherWebsocket {
    fn done(&mut self, py: Python) {
        if let Ok(mut proto) = self.proto.as_ref(py).try_borrow_mut() {
            if let (Some(tx), res) = proto.tx() {
                let _ = tx.send(res);
            }
        }
    }

    fn err(&mut self, py: Python) {
        log::warn!("Application callable raised an exception");
        self.done(py)
    }
}

pub(crate) async fn call_http(
    cb: CallbackWrapper,
    rt: RuntimeRef,
    req: hyper::Request<hyper::Body>,
    scope: Scope
) -> PyResult<Response> {
    let callback = cb.callback.clone();
    let (tx, rx) = oneshot::channel();
    let protocol = HTTPProtocol::new(rt, tx, req);

    Python::with_gil(|py| {
        callback.call1(py, (CallbackWatcherHTTP::new(py, cb, protocol), scope))
    })?;

    match rx.await {
        Ok(res) => {
            Ok(res)
        },
        _ => {
            log::error!("RSGI protocol failure");
            error_proto!()
        }
    }
}

pub(crate) async fn call_ws(
    cb: CallbackWrapper,
    rt: RuntimeRef,
    ws: HyperWebsocket,
    upgrade: UpgradeData,
    scope: Scope
) -> PyResult<(i32, bool)> {
    let callback = cb.callback.clone();
    let (tx, rx) = oneshot::channel();
    let protocol = WebsocketProtocol::new(rt, tx, ws, upgrade);

    Python::with_gil(|py| {
        callback.call1(py, (CallbackWatcherWebsocket::new(py, cb, protocol), scope))
    })?;

    match rx.await {
        Ok(res) => {
            Ok(res)
        },
        _ => {
            log::error!("RSGI protocol failure");
            error_proto!()
        }
    }
}
