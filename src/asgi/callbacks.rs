use hyper::{Body, Request, Response};
use pyo3::prelude::*;
use tokio::sync::oneshot;

use crate::{
    callbacks::CallbackWrapper,
    runtime::RuntimeRef,
    ws::{HyperWebsocket, UpgradeData}
};
use super::{
    errors::{ASGIFlowError, error_flow},
    io::{ASGIHTTPProtocol, ASGIWebsocketProtocol},
    types::ASGIScope as Scope
};


#[pyclass]
pub(crate) struct CallbackWatcherHTTP {
    #[pyo3(get)]
    proto: Py<ASGIHTTPProtocol>,
    #[pyo3(get)]
    event_loop: PyObject,
    #[pyo3(get)]
    context: PyObject
}

impl CallbackWatcherHTTP {
    pub fn new(
        py: Python,
        cb: CallbackWrapper,
        proto: ASGIHTTPProtocol
    ) -> Self {
        Self {
            proto: Py::new(py, proto).unwrap(),
            event_loop: cb.context.event_loop(py).into(),
            context: cb.context.context(py).into(),
        }
    }
}

#[pymethods]
impl CallbackWatcherHTTP {
    fn done(&mut self, py: Python) {
        if let Ok(mut proto) = self.proto.as_ref(py).try_borrow_mut() {
            if let Some(tx) = proto.tx() {
                let mut res = Response::new("Internal server error".into());
                *res.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
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
    proto: Py<ASGIWebsocketProtocol>,
    #[pyo3(get)]
    event_loop: PyObject,
    #[pyo3(get)]
    context: PyObject
}

impl CallbackWatcherWebsocket {
    pub fn new(
        py: Python,
        cb: CallbackWrapper,
        proto: ASGIWebsocketProtocol
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

// NOTE: we cannot use single `impl` function as structs with pyclass won't handle
//       dyn fields easily.
// pub(crate) async fn call(
//     cb: CallbackWrapper,
//     protocol: impl ASGIProtocol + IntoPy<PyObject>,
//     scope: Scope
// ) -> Result<(), ASGIFlowError> {
//     let (tx, rx) = oneshot::channel();
//     let callback = cb.callback.clone();
//     Python::with_gil(|py| {
//         callback.call1(py, (CallbackWatcher::new(py, cb, tx), scope, protocol))
//     })?;

//     match rx.await {
//         Ok(true) => Ok(()),
//         Ok(false) => {
//             log::warn!("Application callable raised an exception");
//             error_flow!()
//         },
//         _ => error_flow!()
//     }
// }

pub(crate) async fn call_http(
    cb: CallbackWrapper,
    rt: RuntimeRef,
    req: Request<Body>,
    scope: Scope
) -> Result<Response<Body>, ASGIFlowError> {
    let callback = cb.callback.clone();
    let (tx, rx) = oneshot::channel();
    let protocol = ASGIHTTPProtocol::new(rt, req, tx);

    Python::with_gil(|py| {
        callback.call1(py, (CallbackWatcherHTTP::new(py, cb, protocol), scope))
    })?;

    match rx.await {
        Ok(res) => {
            Ok(res)
        },
        _ => {
            log::error!("ASGI protocol failure");
            error_flow!()
        }
    }
}

pub(crate) async fn call_ws(
    cb: CallbackWrapper,
    rt: RuntimeRef,
    ws: HyperWebsocket,
    upgrade: UpgradeData,
    scope: Scope
) -> Result<bool, ASGIFlowError> {
    let callback = cb.callback.clone();
    let (tx, rx) = oneshot::channel();
    let protocol = ASGIWebsocketProtocol::new(rt, tx, ws, upgrade);

    Python::with_gil(|py| {
        callback.call1(py, (CallbackWatcherWebsocket::new(py, cb, protocol), scope))
    })?;

    match rx.await {
        Ok(res) => {
            Ok(res)
        },
        _ => {
            log::error!("ASGI protocol failure");
            error_flow!()
        }
    }
}
