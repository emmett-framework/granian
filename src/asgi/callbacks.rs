use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::{
    net::SocketAddr,
    sync::{Arc, OnceLock},
};
use tokio::sync::oneshot;

use super::{
    io::{ASGIHTTPProtocol as HTTPProtocol, ASGIWebsocketProtocol as WebsocketProtocol, WebsocketDetachedTransport},
    utils::{build_scope_http, build_scope_ws, scope_native_parts},
};
use crate::{
    callbacks::ArcCBScheduler,
    http::{response_500, HTTPResponse},
    runtime::RuntimeRef,
    utils::log_application_callable_exception,
    ws::{HyperWebsocket, UpgradeData},
};

macro_rules! callback_impl_done_http {
    ($self:expr) => {
        if let Some(tx) = $self.proto.get().tx() {
            let _ = tx.send(response_500());
        }
    };
}

macro_rules! callback_impl_done_ws {
    ($self:expr) => {
        if let (Some(tx), res) = $self.proto.get().tx() {
            let _ = tx.send(res);
        }
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
    scope: Py<PyDict>,
    aio_taskref: OnceLock<PyObject>,
}

impl CallbackWatcherHTTP {
    pub fn new(py: Python, proto: HTTPProtocol, scope: Bound<PyDict>) -> Self {
        Self {
            proto: Py::new(py, proto).unwrap(),
            scope: scope.unbind(),
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
    scope: Py<PyDict>,
    aio_taskref: OnceLock<PyObject>,
}

impl CallbackWatcherWebsocket {
    pub fn new(py: Python, proto: WebsocketProtocol, scope: Bound<PyDict>) -> Self {
        Self {
            proto: Py::new(py, proto).unwrap(),
            scope: scope.unbind(),
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

#[inline]
pub(crate) fn call_http(
    cb: ArcCBScheduler,
    rt: RuntimeRef,
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    scheme: &str,
    req: hyper::http::request::Parts,
    body: hyper::body::Incoming,
) -> oneshot::Receiver<HTTPResponse> {
    let brt = rt.innerb.clone();
    let (tx, rx) = oneshot::channel();
    let protocol = HTTPProtocol::new(rt, body, tx);
    let scheme: Arc<str> = scheme.into();

    let _ = brt.run(move || {
        scope_native_parts!(
            req,
            server_addr,
            client_addr,
            path,
            query_string,
            version,
            server,
            client
        );
        Python::with_gil(|py| {
            let scope = build_scope_http(py, &req, version, server, client, &scheme, &path, query_string).unwrap();
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
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    scheme: &str,
    ws: HyperWebsocket,
    req: hyper::http::request::Parts,
    upgrade: UpgradeData,
) -> oneshot::Receiver<WebsocketDetachedTransport> {
    let brt = rt.innerb.clone();
    let (tx, rx) = oneshot::channel();
    let protocol = WebsocketProtocol::new(rt, tx, ws, upgrade);
    let scheme: Arc<str> = scheme.into();

    let _ = brt.run(move || {
        scope_native_parts!(
            req,
            server_addr,
            client_addr,
            path,
            query_string,
            version,
            server,
            client
        );
        Python::with_gil(|py| {
            let scope = build_scope_ws(py, &req, version, server, client, &scheme, &path, query_string).unwrap();
            let watcher = Py::new(py, CallbackWatcherWebsocket::new(py, protocol, scope)).unwrap();
            cb.get().schedule(py, watcher.as_any());
        });
    });

    rx
}
