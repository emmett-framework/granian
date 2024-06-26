use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::oneshot;

use super::{
    io::{ASGIHTTPProtocol as HTTPProtocol, ASGIWebsocketProtocol as WebsocketProtocol, WebsocketDetachedTransport},
    utils::{build_scope_http, build_scope_ws, scope_native_parts},
};
use crate::{
    asyncio::PyContext,
    callbacks::{
        callback_impl_loop_err, callback_impl_loop_pytask, callback_impl_loop_run, callback_impl_loop_step,
        callback_impl_loop_wake, callback_impl_run, callback_impl_run_pytask, CallbackWrapper,
    },
    http::{response_500, HTTPResponse},
    runtime::RuntimeRef,
    utils::log_application_callable_exception,
    ws::{HyperWebsocket, UpgradeData},
};

#[pyclass(frozen)]
pub(crate) struct CallbackRunnerHTTP {
    proto: Py<HTTPProtocol>,
    context: PyContext,
    cb: PyObject,
}

impl CallbackRunnerHTTP {
    pub fn new(py: Python, cb: CallbackWrapper, proto: HTTPProtocol, scope: Bound<PyDict>) -> Self {
        let pyproto = Py::new(py, proto).unwrap();
        Self {
            proto: pyproto.clone_ref(py),
            context: cb.context,
            cb: cb.callback.call1(py, (scope, pyproto)).unwrap(),
        }
    }

    callback_impl_run!();
}

#[pymethods]
impl CallbackRunnerHTTP {
    fn _loop_task<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        CallbackTaskHTTP::new(
            py,
            self.cb.clone_ref(py),
            self.proto.clone_ref(py),
            self.context.clone(),
        )?
        .run(py)
    }
}

macro_rules! callback_impl_done_http {
    ($self:expr) => {
        if let Some(tx) = $self.proto.get().tx() {
            let _ = tx.send(response_500());
        }
    };
}

macro_rules! callback_impl_done_err {
    ($self:expr, $err:expr) => {
        $self.done();
        log_application_callable_exception($err);
    };
}

#[pyclass(frozen)]
pub(crate) struct CallbackTaskHTTP {
    proto: Py<HTTPProtocol>,
    context: PyContext,
    pycontext: PyObject,
    cb: PyObject,
}

impl CallbackTaskHTTP {
    pub fn new(py: Python, cb: PyObject, proto: Py<HTTPProtocol>, context: PyContext) -> PyResult<Self> {
        let pyctx = context.context(py);
        Ok(Self {
            proto,
            context,
            pycontext: pyctx.call_method0(pyo3::intern!(py, "copy"))?.into(),
            cb,
        })
    }

    fn done(&self) {
        callback_impl_done_http!(self);
    }

    fn err(&self, err: &PyErr) {
        callback_impl_done_err!(self, err);
    }

    callback_impl_loop_run!();
    callback_impl_loop_err!();
}

#[pymethods]
impl CallbackTaskHTTP {
    fn _loop_step(pyself: PyRef<'_, Self>, py: Python) -> PyResult<()> {
        callback_impl_loop_step!(pyself, py)
    }

    fn _loop_wake(pyself: PyRef<'_, Self>, py: Python, fut: PyObject) -> PyResult<PyObject> {
        callback_impl_loop_wake!(pyself, py, fut)
    }
}

#[pyclass(frozen)]
pub(crate) struct CallbackWrappedRunnerHTTP {
    #[pyo3(get)]
    proto: Py<HTTPProtocol>,
    context: PyContext,
    cb: PyObject,
    #[pyo3(get)]
    scope: PyObject,
}

impl CallbackWrappedRunnerHTTP {
    pub fn new(py: Python, cb: CallbackWrapper, proto: HTTPProtocol, scope: Bound<PyDict>) -> Self {
        Self {
            proto: Py::new(py, proto).unwrap(),
            context: cb.context,
            cb: cb.callback.clone_ref(py),
            scope: scope.into_py(py),
        }
    }

    callback_impl_run_pytask!();
}

#[pymethods]
impl CallbackWrappedRunnerHTTP {
    fn _loop_task<'p>(pyself: PyRef<'_, Self>, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        callback_impl_loop_pytask!(pyself, py)
    }

    fn done(&self) {
        callback_impl_done_http!(self);
    }

    fn err(&self, err: Bound<PyAny>) {
        callback_impl_done_err!(self, &PyErr::from_value_bound(err));
    }
}

#[pyclass(frozen)]
pub(crate) struct CallbackRunnerWebsocket {
    proto: Py<WebsocketProtocol>,
    context: PyContext,
    cb: PyObject,
}

impl CallbackRunnerWebsocket {
    pub fn new(py: Python, cb: CallbackWrapper, proto: WebsocketProtocol, scope: Bound<PyDict>) -> Self {
        let pyproto = Py::new(py, proto).unwrap();
        Self {
            proto: pyproto.clone_ref(py),
            context: cb.context,
            cb: cb.callback.call1(py, (scope, pyproto)).unwrap(),
        }
    }

    callback_impl_run!();
}

#[pymethods]
impl CallbackRunnerWebsocket {
    fn _loop_task<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        CallbackTaskWebsocket::new(
            py,
            self.cb.clone_ref(py),
            self.proto.clone_ref(py),
            self.context.clone(),
        )?
        .run(py)
    }
}

macro_rules! callback_impl_done_ws {
    ($self:expr) => {
        if let (Some(tx), res) = $self.proto.get().tx() {
            let _ = tx.send(res);
        }
    };
}

#[pyclass(frozen)]
pub(crate) struct CallbackTaskWebsocket {
    proto: Py<WebsocketProtocol>,
    context: PyContext,
    pycontext: PyObject,
    cb: PyObject,
}

impl CallbackTaskWebsocket {
    pub fn new(py: Python, cb: PyObject, proto: Py<WebsocketProtocol>, context: PyContext) -> PyResult<Self> {
        let pyctx = context.context(py);
        Ok(Self {
            proto,
            context,
            pycontext: pyctx.call_method0(pyo3::intern!(py, "copy"))?.into(),
            cb,
        })
    }

    fn done(&self) {
        callback_impl_done_ws!(self);
    }

    fn err(&self, err: &PyErr) {
        callback_impl_done_err!(self, err);
    }

    callback_impl_loop_run!();
    callback_impl_loop_err!();
}

#[pymethods]
impl CallbackTaskWebsocket {
    fn _loop_step(pyself: PyRef<'_, Self>, py: Python) -> PyResult<()> {
        callback_impl_loop_step!(pyself, py)
    }

    fn _loop_wake(pyself: PyRef<'_, Self>, py: Python, fut: PyObject) -> PyResult<PyObject> {
        callback_impl_loop_wake!(pyself, py, fut)
    }
}

#[pyclass(frozen)]
pub(crate) struct CallbackWrappedRunnerWebsocket {
    #[pyo3(get)]
    proto: Py<WebsocketProtocol>,
    context: PyContext,
    cb: PyObject,
    #[pyo3(get)]
    scope: PyObject,
}

impl CallbackWrappedRunnerWebsocket {
    pub fn new(py: Python, cb: CallbackWrapper, proto: WebsocketProtocol, scope: Bound<PyDict>) -> Self {
        Self {
            proto: Py::new(py, proto).unwrap(),
            context: cb.context,
            cb: cb.callback.clone_ref(py),
            scope: scope.into_py(py),
        }
    }

    callback_impl_run_pytask!();
}

#[pymethods]
impl CallbackWrappedRunnerWebsocket {
    fn _loop_task<'p>(pyself: PyRef<'_, Self>, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        callback_impl_loop_pytask!(pyself, py)
    }

    fn done(&self) {
        callback_impl_done_ws!(self);
    }

    fn err(&self, err: Bound<PyAny>) {
        callback_impl_done_err!(self, &PyErr::from_value_bound(err));
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

macro_rules! call_impl_http {
    ($func_name:ident, $runner:ident) => {
        #[inline]
        pub(crate) fn $func_name(
            cb: CallbackWrapper,
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
                    let scope =
                        build_scope_http(py, &req, version, server, client, &scheme, &path, query_string).unwrap();
                    let _ = $runner::new(py, cb, protocol, scope).run(py);
                });
            });

            rx
        }
    };
}

macro_rules! call_impl_ws {
    ($func_name:ident, $runner:ident) => {
        #[inline]
        pub(crate) fn $func_name(
            cb: CallbackWrapper,
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
                    let scope =
                        build_scope_ws(py, &req, version, server, client, &scheme, &path, query_string).unwrap();
                    let _ = $runner::new(py, cb, protocol, scope).run(py);
                });
            });

            rx
        }
    };
}

call_impl_http!(call_http, CallbackRunnerHTTP);
call_impl_http!(call_http_pyw, CallbackWrappedRunnerHTTP);
call_impl_ws!(call_ws, CallbackRunnerWebsocket);
call_impl_ws!(call_ws_pyw, CallbackWrappedRunnerWebsocket);
