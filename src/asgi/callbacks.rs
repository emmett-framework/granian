use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::{Arc, OnceLock};
use tokio::sync::{Notify, SetOnce, oneshot};

use super::{
    io::{ASGIHTTPProtocol as HTTPProtocol, ASGIWebsocketProtocol as WebsocketProtocol, WebsocketDetachedTransport},
    utils::{build_scope_http, build_scope_ws},
};
use crate::{
    accounting::build_acct,
    callbacks::ArcCBScheduler,
    http::{HTTPProto, HTTPResponse, response_500},
    net::SockAddr,
    runtime::{RequestRt, Runtime, RuntimeRef},
    scheduler::FlowRef,
    utils::{GuardedReceiver, log_application_callable_exception},
    ws::{HyperWebsocket, UpgradeData},
};

macro_rules! callback_impl_done_http {
    ($self:expr) => {
        if let Some(tx) = $self.proto.get().tx() {
            let _ = tx.send(response_500());
        }
        $self.proto.get().finalize();
    };
}

macro_rules! callback_impl_done_ws {
    ($self:expr) => {
        if let (Some(tx), res) = $self.proto.get().tx() {
            let _ = tx.send(res);
        }
        $self.proto.get().finalize();
    };
}

macro_rules! callback_impl_done_err {
    ($self:expr, $py:expr, $err:expr) => {
        $self.done();
        log_application_callable_exception($py, $err);
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
    #[pyo3(get)]
    ctx: Py<PyAny>,
    aio_taskref: OnceLock<Py<PyAny>>,
}

impl CallbackWatcherHTTP {
    pub fn new(py: Python, proto: HTTPProtocol, scope: Bound<PyDict>, ctx: Option<Py<PyAny>>) -> PyResult<Py<Self>> {
        Py::new(
            py,
            Self {
                proto: Py::new(py, proto)?,
                scope: scope.unbind(),
                ctx: ctx.unwrap_or_else(|| py.None()),
                aio_taskref: OnceLock::new(),
            },
        )
    }
}

#[pymethods]
impl CallbackWatcherHTTP {
    fn done(&self) {
        callback_impl_done_http!(self);
    }

    fn err(&self, py: Python, err: Bound<PyAny>) {
        callback_impl_done_err!(self, py, &PyErr::from_value(err));
    }

    fn taskref(&self, py: Python, task: Py<PyAny>) {
        callback_impl_taskref!(self, py, task);
    }
}

#[pyclass(frozen)]
pub(crate) struct CallbackWatcherWebsocket {
    #[pyo3(get)]
    proto: Py<WebsocketProtocol>,
    #[pyo3(get)]
    scope: Py<PyDict>,
    #[pyo3(get)]
    ctx: Py<PyAny>,
    aio_taskref: OnceLock<Py<PyAny>>,
}

impl CallbackWatcherWebsocket {
    pub fn new(
        py: Python,
        proto: WebsocketProtocol,
        scope: Bound<PyDict>,
        ctx: Option<Py<PyAny>>,
    ) -> PyResult<Py<Self>> {
        Py::new(
            py,
            Self {
                proto: Py::new(py, proto)?,
                scope: scope.unbind(),
                ctx: ctx.unwrap_or_else(|| py.None()),
                aio_taskref: OnceLock::new(),
            },
        )
    }
}

#[pymethods]
impl CallbackWatcherWebsocket {
    fn done(&self) {
        callback_impl_done_ws!(self);
    }

    fn err(&self, py: Python, err: Bound<PyAny>) {
        callback_impl_done_err!(self, py, &PyErr::from_value(err));
    }

    fn taskref(&self, py: Python, task: Py<PyAny>) {
        callback_impl_taskref!(self, py, task);
    }
}

#[inline]
pub(crate) fn call_http(
    cb: ArcCBScheduler,
    rt: RuntimeRef,
    server_addr: SockAddr,
    client_addr: SockAddr,
    scheme: HTTPProto,
    req: hyper::http::request::Parts,
    body: hyper::body::Incoming,
    flow: Option<FlowRef>,
) -> GuardedReceiver<HTTPResponse> {
    let rrt = RequestRt::new(rt.clone(), flow);
    let tracked = flow.is_some();
    let (tx, rx) = oneshot::channel();
    let disconnect_guard = Arc::new(SetOnce::new());
    let protocol = HTTPProtocol::new(rrt.clone(), disconnect_guard.clone(), body, tx);

    rt.spawn_blocking_flow(
        move |py| {
            let (acct, ctx) = build_acct(py, cb.get(), tracked);
            if let Some(acct) = acct {
                protocol.set_acct(acct);
            }
            if let Ok(scope) = build_scope_http(py, req, server_addr, client_addr, scheme)
                && let Ok(watcher) = CallbackWatcherHTTP::new(py, protocol, scope, ctx)
            {
                cb.get().schedule(py, watcher);
            }
        },
        flow.as_ref(),
    );

    GuardedReceiver::new(rx, disconnect_guard)
}

#[inline]
pub(crate) fn call_ws(
    cb: ArcCBScheduler,
    rt: RuntimeRef,
    disconnect_guard: Arc<Notify>,
    server_addr: SockAddr,
    client_addr: SockAddr,
    scheme: HTTPProto,
    ws: HyperWebsocket,
    req: hyper::http::request::Parts,
    upgrade: UpgradeData,
    flow: Option<FlowRef>,
) -> oneshot::Receiver<WebsocketDetachedTransport> {
    let rrt = RequestRt::new(rt.clone(), flow);
    let tracked = flow.is_some();
    let (tx, rx) = oneshot::channel();
    let protocol = WebsocketProtocol::new(rrt, tx, ws, upgrade, disconnect_guard);

    rt.spawn_blocking_flow(
        move |py| {
            let (acct, ctx) = build_acct(py, cb.get(), tracked);
            if let Some(acct) = acct {
                protocol.set_acct(acct);
            }
            if let Ok(scope) = build_scope_ws(py, req, server_addr, client_addr, scheme)
                && let Ok(watcher) = CallbackWatcherWebsocket::new(py, protocol, scope, ctx)
            {
                cb.get().schedule(py, watcher);
            }
        },
        flow.as_ref(),
    );

    rx
}
