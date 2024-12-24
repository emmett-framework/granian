use pyo3::prelude::*;
use std::sync::OnceLock;
use tokio::sync::oneshot;

use super::{
    io::{RSGIHTTPProtocol as HTTPProtocol, RSGIWebsocketProtocol as WebsocketProtocol, WebsocketDetachedTransport},
    types::{PyResponse, PyResponseBody, RSGIHTTPScope as HTTPScope, RSGIWebsocketScope as WebsocketScope},
};
use crate::{
    callbacks::ArcCBScheduler,
    runtime::RuntimeRef,
    utils::log_application_callable_exception,
    ws::{HyperWebsocket, UpgradeData},
};

macro_rules! callback_impl_done_http {
    ($self:expr) => {
        if let Some(tx) = $self.proto.get().tx() {
            let _ = tx.send(PyResponse::Body(PyResponseBody::empty(500, Vec::new())));
        }
    };
}

macro_rules! callback_impl_done_ws {
    ($self:expr) => {
        let _ = $self.proto.get().close(None);
    };
}

macro_rules! callback_impl_done_err {
    ($self:expr, $err:expr) => {
        $self.done();
        log_application_callable_exception($err);
    };
}

macro_rules! callback_impl_taskref {
    ($self:expr, $py:expr, $task:expr) => {
        let _ = $self.aio_taskref.set($task.clone_ref($py));
    };
}

#[pyclass(frozen)]
pub(crate) struct CallbackWatcherHTTP {
    #[pyo3(get)]
    proto: Py<HTTPProtocol>,
    #[pyo3(get)]
    scope: Py<HTTPScope>,
    aio_taskref: OnceLock<PyObject>,
}

impl CallbackWatcherHTTP {
    pub fn new(py: Python, proto: HTTPProtocol, scope: HTTPScope) -> Self {
        Self {
            proto: Py::new(py, proto).unwrap(),
            scope: Py::new(py, scope).unwrap(),
            aio_taskref: OnceLock::new(),
        }
    }
}

#[pymethods]
impl CallbackWatcherHTTP {
    fn done(&self) {
        callback_impl_done_http!(self);
    }

    fn err(&self, err: Bound<PyAny>) {
        callback_impl_done_err!(self, &PyErr::from_value(err));
    }

    fn taskref(&self, py: Python, task: PyObject) {
        callback_impl_taskref!(self, py, task);
    }
}

#[pyclass(frozen)]
pub(crate) struct CallbackWatcherWebsocket {
    #[pyo3(get)]
    proto: Py<WebsocketProtocol>,
    #[pyo3(get)]
    scope: Py<WebsocketScope>,
    aio_taskref: OnceLock<PyObject>,
}

impl CallbackWatcherWebsocket {
    pub fn new(py: Python, proto: WebsocketProtocol, scope: WebsocketScope) -> Self {
        Self {
            proto: Py::new(py, proto).unwrap(),
            scope: Py::new(py, scope).unwrap(),
            aio_taskref: OnceLock::new(),
        }
    }
}

#[pymethods]
impl CallbackWatcherWebsocket {
    fn done(&self) {
        callback_impl_done_ws!(self);
    }

    fn err(&self, err: Bound<PyAny>) {
        callback_impl_done_err!(self, &PyErr::from_value(err));
    }

    fn taskref(&self, py: Python, task: PyObject) {
        callback_impl_taskref!(self, py, task);
    }
}

#[inline]
pub(crate) fn call_http(
    cb: ArcCBScheduler,
    rt: RuntimeRef,
    body: hyper::body::Incoming,
    scope: HTTPScope,
) -> oneshot::Receiver<PyResponse> {
    let brt = rt.innerb.clone();
    let (tx, rx) = oneshot::channel();
    let protocol = HTTPProtocol::new(rt, tx, body);

    let _ = brt.run(move || {
        Python::with_gil(|py| {
            let watcher = Py::new(py, CallbackWatcherHTTP::new(py, protocol, scope)).unwrap();
            cb.get().schedule(py, watcher.as_any());
        });
    });

    rx
}

#[inline]
pub(crate) fn call_ws(
    cb: ArcCBScheduler,
    rt: RuntimeRef,
    ws: HyperWebsocket,
    upgrade: UpgradeData,
    scope: WebsocketScope,
) -> oneshot::Receiver<WebsocketDetachedTransport> {
    let brt = rt.innerb.clone();
    let (tx, rx) = oneshot::channel();
    let protocol = WebsocketProtocol::new(rt, tx, ws, upgrade);

    let _ = brt.run(move || {
        Python::with_gil(|py| {
            let watcher = Py::new(py, CallbackWatcherWebsocket::new(py, protocol, scope)).unwrap();
            cb.get().schedule(py, watcher.as_any());
        });
    });

    rx
}
