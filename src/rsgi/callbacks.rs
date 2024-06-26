use pyo3::prelude::*;
use tokio::sync::oneshot;

use super::{
    io::{RSGIHTTPProtocol as HTTPProtocol, RSGIWebsocketProtocol as WebsocketProtocol, WebsocketDetachedTransport},
    types::{PyResponse, PyResponseBody, RSGIHTTPScope as HTTPScope, RSGIWebsocketScope as WebsocketScope},
};
use crate::{
    asyncio::PyContext,
    callbacks::{
        callback_impl_loop_err, callback_impl_loop_pytask, callback_impl_loop_run, callback_impl_loop_step,
        callback_impl_loop_wake, callback_impl_run, callback_impl_run_pytask, CallbackWrapper,
    },
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
    pub fn new(py: Python, cb: CallbackWrapper, proto: HTTPProtocol, scope: HTTPScope) -> Self {
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
            let _ = tx.send(PyResponse::Body(PyResponseBody::empty(500, Vec::new())));
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
    pub fn new(py: Python, cb: CallbackWrapper, proto: HTTPProtocol, scope: HTTPScope) -> Self {
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
    pub fn new(py: Python, cb: CallbackWrapper, proto: WebsocketProtocol, scope: WebsocketScope) -> Self {
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
        let _ = $self.proto.get().close(None);
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
    pub fn new(py: Python, cb: CallbackWrapper, proto: WebsocketProtocol, scope: WebsocketScope) -> Self {
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

macro_rules! call_impl_http {
    ($func_name:ident, $runner:ident) => {
        #[inline]
        pub(crate) fn $func_name(
            cb: CallbackWrapper,
            rt: RuntimeRef,
            body: hyper::body::Incoming,
            scope: HTTPScope,
        ) -> oneshot::Receiver<PyResponse> {
            let brt = rt.innerb.clone();
            let (tx, rx) = oneshot::channel();
            let protocol = HTTPProtocol::new(rt, tx, body);

            let _ = brt.run(|| {
                Python::with_gil(|py| {
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
            ws: HyperWebsocket,
            upgrade: UpgradeData,
            scope: WebsocketScope,
        ) -> oneshot::Receiver<WebsocketDetachedTransport> {
            let brt = rt.innerb.clone();
            let (tx, rx) = oneshot::channel();
            let protocol = WebsocketProtocol::new(rt, tx, ws, upgrade);

            let _ = brt.run(|| {
                Python::with_gil(|py| {
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
