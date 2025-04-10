use futures::{sink::SinkExt, StreamExt};
use http_body_util::BodyExt;
use hyper::body;
use pyo3::{prelude::*, pybacked::PyBackedStr};
use std::{
    borrow::Cow,
    sync::{atomic, Arc, Mutex, RwLock},
};
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex, Notify};
use tokio_tungstenite::tungstenite::Message;

use super::{
    errors::{error_proto, error_stream},
    types::{PyResponse, PyResponseBody, PyResponseFile},
};
use crate::{
    conversion::FutureResultToPy,
    runtime::{empty_future_into_py, err_future_into_py, future_into_py_futlike, RuntimeRef},
    ws::{HyperWebsocket, UpgradeData, WSRxStream, WSTxStream},
};

pub(crate) type WebsocketDetachedTransport = (i32, bool, Option<WSTxStream>);

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct RSGIHTTPStreamTransport {
    tx: mpsc::UnboundedSender<body::Bytes>,
}

impl RSGIHTTPStreamTransport {
    pub fn new(transport: mpsc::UnboundedSender<body::Bytes>) -> Self {
        Self { tx: transport }
    }
}

// NOTE: the interface doesn't need to be async anymore.
//       This would be a breaking change though; probably requires a major version bump in RSGI
#[pymethods]
impl RSGIHTTPStreamTransport {
    fn send_bytes<'p>(&self, py: Python<'p>, data: Cow<[u8]>) -> PyResult<Bound<'p, PyAny>> {
        let bdata = body::Bytes::from(std::convert::Into::<Box<[u8]>>::into(data));
        match self.tx.send(bdata) {
            Ok(()) => empty_future_into_py(py),
            _ => err_future_into_py(py, error_stream!()),
        }
    }

    fn send_str<'p>(&self, py: Python<'p>, data: String) -> PyResult<Bound<'p, PyAny>> {
        match self.tx.send(body::Bytes::from(data)) {
            Ok(()) => empty_future_into_py(py),
            _ => err_future_into_py(py, error_stream!()),
        }
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct RSGIHTTPProtocol {
    rt: RuntimeRef,
    tx: Mutex<Option<oneshot::Sender<PyResponse>>>,
    disconnect_guard: Arc<Notify>,
    body: Mutex<Option<body::Incoming>>,
    body_stream: Arc<AsyncMutex<Option<http_body_util::BodyStream<body::Incoming>>>>,
    disconnected: Arc<atomic::AtomicBool>,
}

impl RSGIHTTPProtocol {
    pub fn new(
        rt: RuntimeRef,
        tx: oneshot::Sender<PyResponse>,
        body: body::Incoming,
        disconnect_guard: Arc<Notify>,
    ) -> Self {
        Self {
            rt,
            tx: Mutex::new(Some(tx)),
            disconnect_guard,
            body: Mutex::new(Some(body)),
            body_stream: Arc::new(AsyncMutex::new(None)),
            disconnected: Arc::new(atomic::AtomicBool::new(false)),
        }
    }

    pub fn tx(&self) -> Option<oneshot::Sender<PyResponse>> {
        self.tx.lock().unwrap().take()
    }
}

#[pymethods]
impl RSGIHTTPProtocol {
    fn __call__<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        if let Some(body) = self.body.lock().unwrap().take() {
            return future_into_py_futlike(self.rt.clone(), py, async move {
                match body.collect().await {
                    Ok(data) => FutureResultToPy::Bytes(data.to_bytes()),
                    _ => FutureResultToPy::Err(error_stream!()),
                }
            });
        }
        error_proto!()
    }

    fn __aiter__(pyself: Py<Self>, py: Python) -> PyResult<Py<Self>> {
        let inner = pyself.get();
        if let Some(body) = inner.body.lock().unwrap().take() {
            let mut stream = inner.body_stream.blocking_lock();
            *stream = Some(http_body_util::BodyStream::new(body));
            return Ok(pyself.clone_ref(py));
        }
        error_proto!()
    }

    fn __anext__<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        if self.body_stream.blocking_lock().is_none() {
            return Err(pyo3::exceptions::PyStopAsyncIteration::new_err("stream exhausted"));
        }

        let body_stream = self.body_stream.clone();
        future_into_py_futlike(self.rt.clone(), py, async move {
            let guard = &mut *body_stream.lock().await;
            match guard.as_mut().unwrap().next().await {
                Some(chunk) => {
                    let chunk = chunk
                        .map(|buf| buf.into_data().unwrap_or_default())
                        .unwrap_or(body::Bytes::new());
                    FutureResultToPy::Bytes(chunk)
                }
                _ => {
                    _ = guard.take();
                    FutureResultToPy::Bytes(body::Bytes::new())
                }
            }
        })
    }

    fn client_disconnect<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        if self.disconnected.load(atomic::Ordering::Acquire) {
            return empty_future_into_py(py);
        }

        let guard = self.disconnect_guard.clone();
        let state = self.disconnected.clone();
        future_into_py_futlike(self.rt.clone(), py, async move {
            guard.notified().await;
            state.store(true, atomic::Ordering::Release);
            FutureResultToPy::None
        })
    }

    #[pyo3(signature = (status=200, headers=vec![]))]
    fn response_empty(&self, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>) {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            _ = tx.send(PyResponse::Body(PyResponseBody::empty(status, headers)));
        }
    }

    #[pyo3(signature = (status=200, headers=vec![], body=vec![].into()))]
    fn response_bytes(&self, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, body: Cow<[u8]>) {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            _ = tx.send(PyResponse::Body(PyResponseBody::from_bytes(
                status,
                headers,
                body.into(),
            )));
        }
    }

    #[pyo3(signature = (status=200, headers=vec![], body=String::new()))]
    fn response_str(&self, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, body: String) {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            _ = tx.send(PyResponse::Body(PyResponseBody::from_string(status, headers, body)));
        }
    }

    #[pyo3(signature = (status, headers, file))]
    fn response_file(&self, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, file: String) {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            _ = tx.send(PyResponse::File(PyResponseFile::new(status, headers, file)));
        }
    }

    #[pyo3(signature = (status=200, headers=vec![]))]
    fn response_stream<'p>(
        &self,
        py: Python<'p>,
        status: u16,
        headers: Vec<(PyBackedStr, PyBackedStr)>,
    ) -> PyResult<Bound<'p, RSGIHTTPStreamTransport>> {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            let (body_tx, body_rx) = mpsc::unbounded_channel::<body::Bytes>();
            let body_stream = http_body_util::StreamBody::new(
                tokio_stream::wrappers::UnboundedReceiverStream::new(body_rx)
                    .map(body::Frame::data)
                    .map(Result::Ok),
            );
            _ = tx.send(PyResponse::Body(PyResponseBody::new(
                status,
                headers,
                BodyExt::boxed(body_stream),
            )));
            let trx = Py::new(py, RSGIHTTPStreamTransport::new(body_tx))?;
            return Ok(trx.into_bound(py));
        }
        error_proto!()
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct RSGIWebsocketTransport {
    rt: RuntimeRef,
    tx: Arc<AsyncMutex<Option<WSTxStream>>>,
    rx: Arc<AsyncMutex<WSRxStream>>,
}

impl RSGIWebsocketTransport {
    pub fn new(rt: RuntimeRef, tx: Arc<AsyncMutex<Option<WSTxStream>>>, rx: WSRxStream) -> Self {
        Self {
            rt,
            tx,
            rx: Arc::new(AsyncMutex::new(rx)),
        }
    }
}

#[pymethods]
impl RSGIWebsocketTransport {
    fn receive<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let transport = self.rx.clone();
        future_into_py_futlike(self.rt.clone(), py, async move {
            if let Ok(mut stream) = transport.try_lock() {
                while let Some(recv) = stream.next().await {
                    match recv {
                        Ok(Message::Ping(_) | Message::Pong(_)) => {}
                        Ok(message) => return FutureResultToPy::RSGIWSMessage(message),
                        _ => break,
                    }
                }
                return FutureResultToPy::Err(error_stream!());
            }
            FutureResultToPy::Err(error_proto!())
        })
    }

    fn send_bytes<'p>(&self, py: Python<'p>, data: Cow<[u8]>) -> PyResult<Bound<'p, PyAny>> {
        let transport = self.tx.clone();
        let bdata: Box<[u8]> = data.into();

        future_into_py_futlike(self.rt.clone(), py, async move {
            if let Ok(mut guard) = transport.try_lock() {
                if let Some(stream) = &mut *guard {
                    return match stream.send(bdata[..].into()).await {
                        Ok(()) => FutureResultToPy::None,
                        _ => FutureResultToPy::Err(error_stream!()),
                    };
                }
            }
            FutureResultToPy::Err(error_proto!())
        })
    }

    fn send_str<'p>(&self, py: Python<'p>, data: String) -> PyResult<Bound<'p, PyAny>> {
        let transport = self.tx.clone();

        future_into_py_futlike(self.rt.clone(), py, async move {
            if let Ok(mut guard) = transport.try_lock() {
                if let Some(stream) = &mut *guard {
                    return match stream.send(data.into()).await {
                        Ok(()) => FutureResultToPy::None,
                        _ => FutureResultToPy::Err(error_stream!()),
                    };
                }
            }
            FutureResultToPy::Err(error_proto!())
        })
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct RSGIWebsocketProtocol {
    rt: RuntimeRef,
    tx: Mutex<Option<oneshot::Sender<WebsocketDetachedTransport>>>,
    websocket: Arc<AsyncMutex<HyperWebsocket>>,
    upgrade: RwLock<Option<UpgradeData>>,
    transport: Arc<AsyncMutex<Option<WSTxStream>>>,
}

impl RSGIWebsocketProtocol {
    pub fn new(
        rt: RuntimeRef,
        tx: oneshot::Sender<WebsocketDetachedTransport>,
        websocket: HyperWebsocket,
        upgrade: UpgradeData,
    ) -> Self {
        Self {
            rt,
            tx: Mutex::new(Some(tx)),
            websocket: Arc::new(AsyncMutex::new(websocket)),
            upgrade: RwLock::new(Some(upgrade)),
            transport: Arc::new(AsyncMutex::new(None)),
        }
    }

    fn consumed(&self) -> bool {
        self.upgrade.read().unwrap().is_none()
    }
}

#[pymethods]
impl RSGIWebsocketProtocol {
    #[pyo3(signature = (status=None))]
    pub fn close(&self, status: Option<i32>) {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            let mut handle = None;
            if let Ok(mut transport) = self.transport.try_lock() {
                if let Some(transport) = transport.take() {
                    handle = Some(transport);
                }
            }

            let _ = tx.send((status.unwrap_or(0), self.consumed(), handle));
        }
    }

    fn accept<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let rth = self.rt.clone();
        let mut upgrade = self.upgrade.write().unwrap().take().unwrap();
        let transport = self.websocket.clone();
        let itransport = self.transport.clone();
        future_into_py_futlike(self.rt.clone(), py, async move {
            let mut ws = transport.lock().await;
            match upgrade.send(None).await {
                Ok(()) => match (&mut *ws).await {
                    Ok(stream) => {
                        let (stx, srx) = stream.split();
                        {
                            let mut guard = itransport.lock().await;
                            *guard = Some(stx);
                        }
                        FutureResultToPy::RSGIWSAccept(RSGIWebsocketTransport::new(
                            rth.clone(),
                            itransport.clone(),
                            srx,
                        ))
                    }
                    _ => FutureResultToPy::Err(error_proto!()),
                },
                _ => FutureResultToPy::Err(error_proto!()),
            }
        })
    }
}
