use hyper::{Body, Request, Response};
use pyo3::prelude::*;
use pyo3_asyncio::TaskLocals;
use tokio::sync::oneshot;

use super::{
    io::{ASGIHTTPProtocol as HTTPProtocol, ASGIWebsocketProtocol as WebsocketProtocol},
    types::ASGIScope as Scope,
};
use crate::{
    callbacks::{
        callback_impl_loop_err, callback_impl_loop_pytask, callback_impl_loop_run, callback_impl_loop_step,
        callback_impl_loop_wake, callback_impl_run, callback_impl_run_pytask, CallbackWrapper,
    },
    runtime::RuntimeRef,
    ws::{HyperWebsocket, UpgradeData},
};

#[pyclass]
pub(crate) struct CallbackRunnerHTTP {
    proto: Py<HTTPProtocol>,
    context: TaskLocals,
    cb: PyObject,
}

impl CallbackRunnerHTTP {
    pub fn new(py: Python, cb: CallbackWrapper, proto: HTTPProtocol, scope: Scope) -> Self {
        let pyproto = Py::new(py, proto).unwrap();
        Self {
            proto: pyproto.clone(),
            context: cb.context,
            cb: cb.callback.call1(py, (scope, pyproto)).unwrap(),
        }
    }

    callback_impl_run!();
}

#[pymethods]
impl CallbackRunnerHTTP {
    fn _loop_task<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        CallbackTaskHTTP::new(py, self.cb.clone(), self.proto.clone(), self.context.clone())?.run(py)
    }
}

macro_rules! callback_impl_done_http {
    ($self:expr, $py:expr) => {
        if let Ok(mut proto) = $self.proto.as_ref($py).try_borrow_mut() {
            if let Some(tx) = proto.tx() {
                let mut res = Response::new("Internal server error".into());
                *res.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
                let _ = tx.send(res);
            }
        }
    };
}

macro_rules! callback_impl_done_err {
    ($self:expr, $py:expr) => {
        log::warn!("Application callable raised an exception");
        $self.done($py)
    };
}

#[pyclass]
pub(crate) struct CallbackTaskHTTP {
    proto: Py<HTTPProtocol>,
    context: TaskLocals,
    pycontext: PyObject,
    cb: PyObject,
}

impl CallbackTaskHTTP {
    pub fn new(py: Python, cb: PyObject, proto: Py<HTTPProtocol>, context: TaskLocals) -> PyResult<Self> {
        let pyctx = context.context(py);
        Ok(Self {
            proto,
            context,
            pycontext: pyctx.call_method0(pyo3::intern!(py, "copy"))?.into(),
            cb,
        })
    }

    fn done(&self, py: Python) {
        callback_impl_done_http!(self, py);
    }

    fn err(&self, py: Python) {
        callback_impl_done_err!(self, py);
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

#[pyclass]
pub(crate) struct CallbackWrappedRunnerHTTP {
    #[pyo3(get)]
    proto: Py<HTTPProtocol>,
    context: TaskLocals,
    cb: PyObject,
    #[pyo3(get)]
    scope: PyObject,
}

impl CallbackWrappedRunnerHTTP {
    pub fn new(py: Python, cb: CallbackWrapper, proto: HTTPProtocol, scope: Scope) -> Self {
        Self {
            proto: Py::new(py, proto).unwrap(),
            context: cb.context,
            cb: cb.callback,
            scope: scope.into_py(py),
        }
    }

    callback_impl_run_pytask!();
}

#[pymethods]
impl CallbackWrappedRunnerHTTP {
    fn _loop_task<'p>(pyself: PyRef<'_, Self>, py: Python<'p>) -> PyResult<&'p PyAny> {
        callback_impl_loop_pytask!(pyself, py)
    }

    fn done(&self, py: Python) {
        callback_impl_done_http!(self, py);
    }

    fn err(&self, py: Python) {
        callback_impl_done_err!(self, py);
    }
}

#[pyclass]
pub(crate) struct CallbackRunnerWebsocket {
    proto: Py<WebsocketProtocol>,
    context: TaskLocals,
    cb: PyObject,
}

impl CallbackRunnerWebsocket {
    pub fn new(py: Python, cb: CallbackWrapper, proto: WebsocketProtocol, scope: Scope) -> Self {
        let pyproto = Py::new(py, proto).unwrap();
        Self {
            proto: pyproto.clone(),
            context: cb.context,
            cb: cb.callback.call1(py, (scope, pyproto)).unwrap(),
        }
    }

    callback_impl_run!();
}

#[pymethods]
impl CallbackRunnerWebsocket {
    fn _loop_task<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        CallbackTaskWebsocket::new(py, self.cb.clone(), self.proto.clone(), self.context.clone())?.run(py)
    }
}

macro_rules! callback_impl_done_ws {
    ($self:expr, $py:expr) => {
        if let Ok(mut proto) = $self.proto.as_ref($py).try_borrow_mut() {
            if let (Some(tx), res) = proto.tx() {
                let _ = tx.send(res);
            }
        }
    };
}

#[pyclass]
pub(crate) struct CallbackTaskWebsocket {
    proto: Py<WebsocketProtocol>,
    context: TaskLocals,
    pycontext: PyObject,
    cb: PyObject,
}

impl CallbackTaskWebsocket {
    pub fn new(py: Python, cb: PyObject, proto: Py<WebsocketProtocol>, context: TaskLocals) -> PyResult<Self> {
        let pyctx = context.context(py);
        Ok(Self {
            proto,
            context,
            pycontext: pyctx.call_method0(pyo3::intern!(py, "copy"))?.into(),
            cb,
        })
    }

    fn done(&self, py: Python) {
        callback_impl_done_ws!(self, py);
    }

    fn err(&self, py: Python) {
        callback_impl_done_err!(self, py);
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

#[pyclass]
pub(crate) struct CallbackWrappedRunnerWebsocket {
    #[pyo3(get)]
    proto: Py<WebsocketProtocol>,
    context: TaskLocals,
    cb: PyObject,
    #[pyo3(get)]
    scope: PyObject,
}

impl CallbackWrappedRunnerWebsocket {
    pub fn new(py: Python, cb: CallbackWrapper, proto: WebsocketProtocol, scope: Scope) -> Self {
        Self {
            proto: Py::new(py, proto).unwrap(),
            context: cb.context,
            cb: cb.callback,
            scope: scope.into_py(py),
        }
    }

    callback_impl_run_pytask!();
}

#[pymethods]
impl CallbackWrappedRunnerWebsocket {
    fn _loop_task<'p>(pyself: PyRef<'_, Self>, py: Python<'p>) -> PyResult<&'p PyAny> {
        callback_impl_loop_pytask!(pyself, py)
    }

    fn done(&self, py: Python) {
        callback_impl_done_ws!(self, py);
    }

    fn err(&self, py: Python) {
        callback_impl_done_err!(self, py);
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

macro_rules! call_impl_rtb_http {
    ($func_name:ident, $runner:ident) => {
        pub(crate) fn $func_name(
            cb: CallbackWrapper,
            rt: RuntimeRef,
            req: Request<Body>,
            scope: Scope,
        ) -> oneshot::Receiver<Response<Body>> {
            let (tx, rx) = oneshot::channel();
            let protocol = HTTPProtocol::new(rt, req, tx);

            Python::with_gil(|py| {
                let _ = $runner::new(py, cb, protocol, scope).run(py);
            });

            rx
        }
    };
}

macro_rules! call_impl_rtt_http {
    ($func_name:ident, $runner:ident) => {
        pub(crate) fn $func_name(
            cb: CallbackWrapper,
            rt: RuntimeRef,
            req: Request<Body>,
            scope: Scope,
        ) -> oneshot::Receiver<Response<Body>> {
            let (tx, rx) = oneshot::channel();
            let protocol = HTTPProtocol::new(rt, req, tx);

            tokio::task::spawn_blocking(move || {
                Python::with_gil(|py| {
                    let _ = $runner::new(py, cb, protocol, scope).run(py);
                });
            });

            rx
        }
    };
}

macro_rules! call_impl_rtb_ws {
    ($func_name:ident, $runner:ident) => {
        pub(crate) fn $func_name(
            cb: CallbackWrapper,
            rt: RuntimeRef,
            ws: HyperWebsocket,
            upgrade: UpgradeData,
            scope: Scope,
        ) -> oneshot::Receiver<bool> {
            let (tx, rx) = oneshot::channel();
            let protocol = WebsocketProtocol::new(rt, tx, ws, upgrade);

            Python::with_gil(|py| {
                let _ = $runner::new(py, cb, protocol, scope).run(py);
            });

            rx
        }
    };
}

macro_rules! call_impl_rtt_ws {
    ($func_name:ident, $runner:ident) => {
        pub(crate) fn $func_name(
            cb: CallbackWrapper,
            rt: RuntimeRef,
            ws: HyperWebsocket,
            upgrade: UpgradeData,
            scope: Scope,
        ) -> oneshot::Receiver<bool> {
            let (tx, rx) = oneshot::channel();
            let protocol = WebsocketProtocol::new(rt, tx, ws, upgrade);

            tokio::task::spawn_blocking(move || {
                Python::with_gil(|py| {
                    let _ = $runner::new(py, cb, protocol, scope).run(py);
                });
            });

            rx
        }
    };
}

call_impl_rtb_http!(call_rtb_http, CallbackRunnerHTTP);
call_impl_rtb_http!(call_rtb_http_pyw, CallbackWrappedRunnerHTTP);
call_impl_rtt_http!(call_rtt_http, CallbackRunnerHTTP);
call_impl_rtt_http!(call_rtt_http_pyw, CallbackWrappedRunnerHTTP);
call_impl_rtb_ws!(call_rtb_ws, CallbackRunnerWebsocket);
call_impl_rtb_ws!(call_rtb_ws_pyw, CallbackWrappedRunnerWebsocket);
call_impl_rtt_ws!(call_rtt_ws, CallbackRunnerWebsocket);
call_impl_rtt_ws!(call_rtt_ws_pyw, CallbackWrappedRunnerWebsocket);
