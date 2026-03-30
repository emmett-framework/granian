use pyo3::prelude::*;
use std::sync::Arc;

use crate::http::{HTTPProto, HTTPRequest};
use crate::net::SockAddr;
use crate::runtime::Runtime;

use crate::rsgi::types::RSGIHTTPScope as HTTPScope;

use super::io::RSGI2HTTPProtocol;

#[pyclass(frozen, module = "granian._granian")]
pub(super) struct PyAbortHandle {
    handle: tokio::task::AbortHandle,
}

impl PyAbortHandle {
    pub(super) fn new(handle: tokio::task::AbortHandle) -> Self {
        Self { handle }
    }
}

#[pymethods]
impl PyAbortHandle {
    fn __call__(&self) {
        self.handle.abort();
    }
}

#[derive(Clone)]
pub(super) struct CallbackImpl {
    rt: crate::runtime::RuntimeRef,
    on_request: Arc<Py<PyAny>>,
    // on_data: Arc<Py<PyAny>>,
    // on_disconnect: Arc<Py<PyAny>>,
}

impl CallbackImpl {
    pub(super) fn from_app(rt: crate::runtime::RuntimeRef, app: &super::app::RSGIApp) -> Self {
        Self {
            rt,
            on_request: app.on_request.clone(),
            // on_data: app.on_data.clone(),
            // on_disconnect: app.on_disconnect.clone(),
        }
    }

    // fn proto_impl(&self) -> CallbackProtoImplHTTP {
    //     CallbackProtoImplHTTP { rt: self.rt.clone() }
    // }
}

// #[derive(Clone)]
// pub(super) struct CallbackProtoImplHTTP {
//     pub rt: crate::runtime::RuntimeRef,
//     pub on_data: Arc<Py<PyAny>>,
//     pub on_disconnect: Arc<Py<PyAny>>,
// }

macro_rules! build_scope {
    ($cls:ty, $server_addr:expr, $client_addr:expr, $req:expr, $scheme:expr) => {
        <$cls>::new(
            $req.version,
            $scheme,
            $req.uri,
            $req.method,
            $server_addr,
            $client_addr,
            $req.headers,
        )
    };
}

impl CallbackImpl {
    pub(super) fn http(
        &self,
        disconnect_guard: Arc<tokio::sync::Notify>,
        server_addr: SockAddr,
        client_addr: SockAddr,
        scheme: HTTPProto,
        request: HTTPRequest,
    ) -> tokio::sync::oneshot::Receiver<crate::rsgi::types::PyResponse> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let (parts, body) = request.into_parts();
        let scope = build_scope!(HTTPScope, server_addr, client_addr, parts, scheme);
        let protocol = RSGI2HTTPProtocol::new(self.rt.clone(), tx, body, disconnect_guard);
        let cb = self.on_request.clone();
        self.rt.spawn_blocking(move |py| {
            _ = cb.call1(py, (protocol, scope));
        });

        rx
    }

    pub(super) fn ws(&self) {
        todo!()
    }

    // pub(super) fn call(&self, disconnect_guard: Arc<tokio::sync::Notify>, body: hyper::body::Incoming, scope: crate::rsgi::types::RSGIHTTPScope) -> tokio::sync::oneshot::Receiver<crate::rsgi::types::PyResponse> {
    //     let (tx, rx) = tokio::sync::oneshot::channel();
    //     // protocol
    //     let protocol = RSGI2HTTPProtocol::new(self.rt.clone(), tx, body, disconnect_guard);
    //     // call
    //     let cb = self.on_request.clone();
    //     self.rt.spawn_blocking(move |py| {
    //         _ = cb.call1(py, (protocol, scope));
    //     });

    //     rx
    // }
}
