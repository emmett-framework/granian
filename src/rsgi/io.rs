use futures::{sink::SinkExt, StreamExt, TryStreamExt};
use http_body_util::BodyExt;
use hyper::body;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};
use std::{borrow::Cow, sync::Arc};
use tokio::sync::{mpsc, oneshot, Mutex};
use tungstenite::Message;

use super::{
    errors::{error_proto, error_stream},
    types::{PyResponse, PyResponseBody, PyResponseFile},
};
use crate::{
    conversion::BytesToPy,
    http::HTTPRequest,
    runtime::{future_into_py_futlike, future_into_py_iter, Runtime, RuntimeRef},
    ws::{HyperWebsocket, UpgradeData, WSRxStream, WSStream, WSTxStream},
};

pub(crate) type WebsocketDetachedTransport = (i32, bool, Option<tokio::task::JoinHandle<()>>);

#[pyclass(module = "granian._granian")]
pub(crate) struct RSGIHTTPStreamTransport {
    rt: RuntimeRef,
    tx: mpsc::Sender<Result<body::Bytes, anyhow::Error>>,
}

impl RSGIHTTPStreamTransport {
    pub fn new(rt: RuntimeRef, transport: mpsc::Sender<Result<body::Bytes, anyhow::Error>>) -> Self {
        Self { rt, tx: transport }
    }
}

#[pymethods]
impl RSGIHTTPStreamTransport {
    fn send_bytes<'p>(&self, py: Python<'p>, data: Cow<[u8]>) -> PyResult<&'p PyAny> {
        let transport = self.tx.clone();
        let bdata: Box<[u8]> = data.into();
        future_into_py_futlike(self.rt.clone(), py, async move {
            match transport.send(Ok(body::Bytes::from(bdata))).await {
                Ok(()) => Ok(()),
                _ => error_stream!(),
            }
        })
    }

    fn send_str<'p>(&self, py: Python<'p>, data: String) -> PyResult<&'p PyAny> {
        let transport = self.tx.clone();
        future_into_py_futlike(self.rt.clone(), py, async move {
            match transport.send(Ok(body::Bytes::from(data))).await {
                Ok(()) => Ok(()),
                _ => error_stream!(),
            }
        })
    }
}

#[pyclass(module = "granian._granian")]
pub(crate) struct RSGIHTTPProtocol {
    rt: RuntimeRef,
    tx: Option<oneshot::Sender<PyResponse>>,
    body: Option<body::Incoming>,
    body_stream: Option<Arc<Mutex<http_body_util::BodyStream<body::Incoming>>>>,
}

impl RSGIHTTPProtocol {
    pub fn new(rt: RuntimeRef, tx: oneshot::Sender<PyResponse>, request: HTTPRequest) -> Self {
        Self {
            rt,
            tx: Some(tx),
            body: Some(request.into_body()),
            body_stream: None,
        }
    }

    pub fn tx(&mut self) -> Option<oneshot::Sender<PyResponse>> {
        self.tx.take()
    }
}

#[pymethods]
impl RSGIHTTPProtocol {
    fn __call__<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        if let Some(body) = self.body.take() {
            return future_into_py_iter(self.rt.clone(), py, async move {
                let body = body
                    .collect()
                    .await
                    .map_err(|_err| pyo3::exceptions::PyRuntimeError::new_err("err"))?;
                Ok(BytesToPy(body.to_bytes()))
            });
        }
        error_proto!()
    }

    fn __aiter__(mut pyself: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        if let Some(body) = pyself.body.take() {
            pyself.body_stream = Some(Arc::new(Mutex::new(http_body_util::BodyStream::new(body))));
        }
        pyself
    }

    fn __anext__<'p>(&mut self, py: Python<'p>) -> PyResult<Option<&'p PyAny>> {
        if let Some(body_ref) = &self.body_stream {
            let body_ref = body_ref.clone();
            let fut = future_into_py_iter(self.rt.clone(), py, async move {
                let mut bodym = body_ref.lock().await;
                let body = &mut *bodym;
                match body.next().await {
                    Some(chunk) => {
                        let chunk = chunk
                            .map(|buf| buf.into_data().unwrap_or_default())
                            .unwrap_or(body::Bytes::new());
                        Ok(BytesToPy(chunk))
                    }
                    _ => Err(pyo3::exceptions::PyStopAsyncIteration::new_err("stream exhausted")),
                }
            })?;
            return Ok(Some(fut));
        }
        error_proto!()
    }

    #[pyo3(signature = (status=200, headers=vec![]))]
    fn response_empty(&mut self, status: u16, headers: Vec<(String, String)>) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(PyResponse::Body(PyResponseBody::empty(status, headers)));
        }
    }

    #[pyo3(signature = (status=200, headers=vec![], body=vec![].into()))]
    fn response_bytes(&mut self, status: u16, headers: Vec<(String, String)>, body: Cow<[u8]>) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(PyResponse::Body(PyResponseBody::from_bytes(status, headers, body)));
        }
    }

    #[pyo3(signature = (status=200, headers=vec![], body=String::new()))]
    fn response_str(&mut self, status: u16, headers: Vec<(String, String)>, body: String) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(PyResponse::Body(PyResponseBody::from_string(status, headers, body)));
        }
    }

    #[pyo3(signature = (status, headers, file))]
    fn response_file(&mut self, status: u16, headers: Vec<(String, String)>, file: String) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(PyResponse::File(PyResponseFile::new(status, headers, file)));
        }
    }

    #[pyo3(signature = (status=200, headers=vec![]))]
    fn response_stream<'p>(
        &mut self,
        py: Python<'p>,
        status: u16,
        headers: Vec<(String, String)>,
    ) -> PyResult<&'p PyAny> {
        if let Some(tx) = self.tx.take() {
            let (body_tx, body_rx) = mpsc::channel::<Result<body::Bytes, anyhow::Error>>(1);
            let body_stream = http_body_util::StreamBody::new(
                tokio_stream::wrappers::ReceiverStream::new(body_rx).map_ok(hyper::body::Frame::data),
            );
            let _ = tx.send(PyResponse::Body(PyResponseBody::new(
                status,
                headers,
                BodyExt::boxed(BodyExt::map_err(body_stream, std::convert::Into::into)),
            )));
            let trx = Py::new(py, RSGIHTTPStreamTransport::new(self.rt.clone(), body_tx))?;
            return Ok(trx.into_ref(py));
        }
        error_proto!()
    }
}

#[pyclass(module = "granian._granian")]
pub(crate) struct RSGIWebsocketTransport {
    rt: RuntimeRef,
    tx: Arc<Mutex<WSTxStream>>,
    rx: Arc<Mutex<WSRxStream>>,
    closed: bool,
}

impl RSGIWebsocketTransport {
    pub fn new(rt: RuntimeRef, transport: WSStream) -> Self {
        let (tx, rx) = transport.split();
        Self {
            rt,
            tx: Arc::new(Mutex::new(tx)),
            rx: Arc::new(Mutex::new(rx)),
            closed: false,
        }
    }

    pub fn close(&mut self) -> Option<tokio::task::JoinHandle<()>> {
        if self.closed {
            return None;
        }
        self.closed = true;

        let tx = self.tx.clone();
        let handle = self.rt.spawn(async move {
            if let Ok(mut tx) = tx.try_lock() {
                if let Err(err) = tx.close().await {
                    log::info!("Failed to close websocket with error {:?}", err);
                }
            }
        });
        Some(handle)
    }
}

#[pymethods]
impl RSGIWebsocketTransport {
    fn receive<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let transport = self.rx.clone();
        future_into_py_futlike(self.rt.clone(), py, async move {
            if let Ok(mut stream) = transport.try_lock() {
                while let Some(recv) = stream.next().await {
                    match recv {
                        Ok(Message::Ping(_)) => continue,
                        Ok(message) => return message_into_py(message),
                        _ => break,
                    }
                }
                return error_stream!();
            }
            error_proto!()
        })
    }

    fn send_bytes<'p>(&self, py: Python<'p>, data: Cow<[u8]>) -> PyResult<&'p PyAny> {
        let transport = self.tx.clone();
        let bdata: Box<[u8]> = data.into();
        future_into_py_iter(self.rt.clone(), py, async move {
            if let Ok(mut stream) = transport.try_lock() {
                return match stream.send(bdata[..].into()).await {
                    Ok(()) => Ok(()),
                    _ => error_stream!(),
                };
            }
            error_proto!()
        })
    }

    fn send_str<'p>(&self, py: Python<'p>, data: String) -> PyResult<&'p PyAny> {
        let transport = self.tx.clone();
        future_into_py_iter(self.rt.clone(), py, async move {
            if let Ok(mut stream) = transport.try_lock() {
                return match stream.send(Message::Text(data)).await {
                    Ok(()) => Ok(()),
                    _ => error_stream!(),
                };
            }
            error_proto!()
        })
    }
}

#[pyclass(module = "granian._granian")]
pub(crate) struct RSGIWebsocketProtocol {
    rt: RuntimeRef,
    tx: Option<oneshot::Sender<WebsocketDetachedTransport>>,
    websocket: Arc<Mutex<HyperWebsocket>>,
    upgrade: Option<UpgradeData>,
    transport: Arc<Mutex<Option<Py<RSGIWebsocketTransport>>>>,
    status: i32,
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
            tx: Some(tx),
            websocket: Arc::new(Mutex::new(websocket)),
            upgrade: Some(upgrade),
            transport: Arc::new(Mutex::new(None)),
            status: 0,
        }
    }

    fn consumed(&self) -> bool {
        self.upgrade.is_none()
    }
}

enum WebsocketMessageType {
    Close = 0,
    Bytes = 1,
    Text = 2,
}

#[pyclass]
struct WebsocketInboundCloseMessage {
    #[pyo3(get)]
    kind: usize,
}

impl WebsocketInboundCloseMessage {
    pub fn new() -> Self {
        Self {
            kind: WebsocketMessageType::Close as usize,
        }
    }
}

#[pyclass]
struct WebsocketInboundBytesMessage {
    #[pyo3(get)]
    kind: usize,
    #[pyo3(get)]
    data: Py<PyBytes>,
}

impl WebsocketInboundBytesMessage {
    pub fn new(data: Py<PyBytes>) -> Self {
        Self {
            kind: WebsocketMessageType::Bytes as usize,
            data,
        }
    }
}

#[pyclass]
struct WebsocketInboundTextMessage {
    #[pyo3(get)]
    kind: usize,
    #[pyo3(get)]
    data: Py<PyString>,
}

impl WebsocketInboundTextMessage {
    pub fn new(data: Py<PyString>) -> Self {
        Self {
            kind: WebsocketMessageType::Text as usize,
            data,
        }
    }
}

#[pymethods]
impl RSGIWebsocketProtocol {
    #[pyo3(signature = (status=None))]
    pub fn close(&mut self, py: Python, status: Option<i32>) -> PyResult<()> {
        self.status = status.unwrap_or(0);
        if let Some(tx) = self.tx.take() {
            let mut handle = None;
            if let Ok(mut transport) = self.transport.try_lock() {
                if let Some(transport) = transport.take() {
                    if let Ok(mut trx) = transport.try_borrow_mut(py) {
                        handle = trx.close();
                    }
                }
            }

            let _ = tx.send((self.status, self.consumed(), handle));
        }
        Ok(())
    }

    fn accept<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let rth = self.rt.clone();
        let mut upgrade = self.upgrade.take().unwrap();
        let transport = self.websocket.clone();
        let itransport = self.transport.clone();
        future_into_py_iter(self.rt.clone(), py, async move {
            let mut ws = transport.lock().await;
            match upgrade.send().await {
                Ok(()) => match (&mut *ws).await {
                    Ok(stream) => {
                        let mut trx = itransport.lock().await;
                        Ok(Python::with_gil(|py| {
                            let pytransport = Py::new(py, RSGIWebsocketTransport::new(rth, stream)).unwrap();
                            *trx = Some(pytransport.clone());
                            pytransport
                        }))
                    }
                    _ => error_proto!(),
                },
                _ => error_proto!(),
            }
        })
    }
}

#[inline(always)]
fn message_into_py(message: Message) -> PyResult<PyObject> {
    match message {
        Message::Binary(message) => Ok(Python::with_gil(|py| {
            WebsocketInboundBytesMessage::new(PyBytes::new(py, &message).into()).into_py(py)
        })),
        Message::Text(message) => Ok(Python::with_gil(|py| {
            WebsocketInboundTextMessage::new(PyString::new(py, &message).into()).into_py(py)
        })),
        Message::Close(_) => Ok(Python::with_gil(|py| WebsocketInboundCloseMessage::new().into_py(py))),
        v => {
            log::warn!("Unsupported websocket message received {:?}", v);
            error_proto!()
        }
    }
}
