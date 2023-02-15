use futures::{sink::SinkExt, stream::{SplitSink, SplitStream, StreamExt}};
use hyper::{Body, Request};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};
use std::sync::Arc;
use tokio_tungstenite::WebSocketStream;
use tokio::sync::{oneshot, Mutex};
use tungstenite::Message;

use crate::{
    runtime::{Runtime, RuntimeRef, future_into_py_iter, future_into_py_futlike},
    ws::{HyperWebsocket, UpgradeData}
};
use super::{
    errors::{error_proto, error_stream},
    types::{PyResponse, PyResponseBytes, PyResponseFile}
};


#[pyclass(module="granian._granian")]
pub(crate) struct RSGIHTTPProtocol {
    rt: RuntimeRef,
    tx: Option<oneshot::Sender<super::types::PyResponse>>,
    request: Arc<Mutex<Request<Body>>>
}

impl RSGIHTTPProtocol {
    pub fn new(
        rt: RuntimeRef,
        tx: oneshot::Sender<super::types::PyResponse>,
        request: Request<Body>
    ) -> Self {
        Self {
            rt,
            tx: Some(tx),
            request: Arc::new(Mutex::new(request))
        }
    }

    pub fn tx(&mut self) -> Option<oneshot::Sender<super::types::PyResponse>> {
        self.tx.take()
    }
}

#[pymethods]
impl RSGIHTTPProtocol {
    fn __call__<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let req_ref = self.request.clone();
        future_into_py_iter(self.rt.clone(), py, async move {
            let mut req = req_ref.lock().await;
            let body = hyper::body::to_bytes(&mut *req).await.unwrap();
            Ok(Python::with_gil(|py| {
                PyBytes::new(py, &body[..]).as_ref().to_object(py)
            }))
        })
    }

    #[args(status="200", headers="vec![]")]
    fn response_empty(&mut self, status: u16, headers: Vec<(String, String)>) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(
                PyResponse::Bytes(PyResponseBytes::empty(status, headers))
            );
        }
    }

    #[args(status="200", headers="vec![]")]
    fn response_bytes(&mut self, status: u16, headers: Vec<(String, String)>, body: Vec<u8>) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(
                PyResponse::Bytes(PyResponseBytes::from_bytes(status, headers, body))
            );
        }
    }

    #[args(status="200", headers="vec![]")]
    fn response_str(&mut self, status: u16, headers: Vec<(String, String)>, body: String) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(
                PyResponse::Bytes(PyResponseBytes::from_string(status, headers, body))
            );
        }
    }

    #[args(status="200", headers="vec![]")]
    fn response_file(&mut self, status: u16, headers: Vec<(String, String)>, file: String) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(
                PyResponse::File(PyResponseFile::new(status, headers, file))
            );
        }
    }
}

#[pyclass(module="granian._granian")]
pub(crate) struct RSGIWebsocketTransport {
    rt: RuntimeRef,
    tx: Arc<Mutex<SplitSink<WebSocketStream<hyper::upgrade::Upgraded>, Message>>>,
    rx: Arc<Mutex<SplitStream<WebSocketStream<hyper::upgrade::Upgraded>>>>
}

impl RSGIWebsocketTransport {
    pub fn new(
        rt: RuntimeRef,
        transport: WebSocketStream<hyper::upgrade::Upgraded>
    ) -> Self {
        let (tx, rx) = transport.split();
        Self { rt: rt, tx: Arc::new(Mutex::new(tx)), rx: Arc::new(Mutex::new(rx)) }
    }

    pub fn close(&self) {
        let stream = self.tx.clone();
        self.rt.spawn(async move {
            if let Ok(mut stream) = stream.try_lock() {
                let _ = stream.close().await;
            }
        });
    }
}

#[pymethods]
impl RSGIWebsocketTransport {
    fn receive<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let transport = self.rx.clone();
        future_into_py_futlike(self.rt.clone(), py, async move {
            if let Ok(mut stream) = transport.try_lock() {
                loop {
                    match stream.next().await {
                        Some(recv) => {
                            match recv {
                                Ok(Message::Ping(_)) => {
                                    continue
                                },
                                Ok(message) => {
                                    return message_into_py(message)
                                },
                                _ => {
                                    break
                                }
                            }
                        },
                        _ => {
                            break
                        }
                    }
                }
                return error_stream!()
            }
            error_proto!()
        })
    }

    fn send_bytes<'p>(&self, py: Python<'p>, data: Vec<u8>) -> PyResult<&'p PyAny> {
        let transport = self.tx.clone();
        future_into_py_iter(self.rt.clone(), py, async move {
            if let Ok(mut stream) = transport.try_lock() {
                return match stream.send(Message::Binary(data)).await {
                    Ok(_) => Ok(()),
                    _ => error_stream!()
                }
            }
            error_proto!()
        })
    }

    fn send_str<'p>(&self, py: Python<'p>, data: String) -> PyResult<&'p PyAny> {
        let transport = self.tx.clone();
        future_into_py_iter(self.rt.clone(), py, async move {
            if let Ok(mut stream) = transport.try_lock() {
                return match stream.send(Message::Text(data)).await {
                    Ok(_) => Ok(()),
                    _ => error_stream!()
                }
            }
            error_proto!()
        })
    }
}

#[pyclass(module="granian._granian")]
pub(crate) struct RSGIWebsocketProtocol {
    rt: RuntimeRef,
    tx: Option<oneshot::Sender<(i32, bool)>>,
    websocket: Arc<Mutex<HyperWebsocket>>,
    upgrade: Option<UpgradeData>,
    transport: Arc<Mutex<Option<Py<RSGIWebsocketTransport>>>>,
    status: i32
}

impl RSGIWebsocketProtocol {
    pub fn new(
        rt: RuntimeRef,
        tx: oneshot::Sender<(i32, bool)>,
        websocket: HyperWebsocket,
        upgrade: UpgradeData
    ) -> Self {
        Self {
            rt,
            tx: Some(tx),
            websocket: Arc::new(Mutex::new(websocket)),
            upgrade: Some(upgrade),
            transport: Arc::new(Mutex::new(None)),
            status: 0
        }
    }

    fn consumed(&self) -> bool {
        match &self.upgrade {
            Some(_) => false,
            _ => true
        }
    }

    pub fn tx(&mut self) -> (Option<oneshot::Sender<(i32, bool)>>, (i32, bool)) {
        (self.tx.take(), (self.status, self.consumed()))
    }
}

enum WebsocketMessageType {
    Close = 0,
    Bytes = 1,
    Text = 2
}

#[pyclass]
struct WebsocketInboundCloseMessage {
    #[pyo3(get)]
    kind: usize
}

impl WebsocketInboundCloseMessage {
    pub fn new() -> Self {
        Self { kind: WebsocketMessageType::Close as usize }
    }
}

#[pyclass]
struct WebsocketInboundBytesMessage {
    #[pyo3(get)]
    kind: usize,
    #[pyo3(get)]
    data: Py<PyBytes>
}

impl WebsocketInboundBytesMessage {
    pub fn new(data:Py<PyBytes>) -> Self {
        Self { kind: WebsocketMessageType::Bytes as usize, data: data }
    }
}

#[pyclass]
struct WebsocketInboundTextMessage {
    #[pyo3(get)]
    kind: usize,
    #[pyo3(get)]
    data: Py<PyString>
}

impl WebsocketInboundTextMessage {
    pub fn new(data: Py<PyString>) -> Self {
        Self { kind: WebsocketMessageType::Text as usize, data: data }
    }
}

#[pymethods]
impl RSGIWebsocketProtocol {
    #[args(status="None")]
    fn close(&mut self, py: Python, status: Option<i32>) -> PyResult<()> {
        self.status = status.unwrap_or(0);
        if let Some(tx) = self.tx.take() {
            if let Ok(mut transport) = self.transport.try_lock() {
                if let Some(transport) = transport.take() {
                    if let Ok(trx) = transport.try_borrow_mut(py) {
                        trx.close();
                    }
                }
            }

            let _ = tx.send((self.status, self.consumed()));
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
                Ok(_) => {
                    match (&mut *ws).await {
                        Ok(stream) => {
                            let mut trx = itransport.lock().await;
                            Ok(Python::with_gil(|py| {
                                let pytransport = Py::new(
                                    py,
                                    RSGIWebsocketTransport::new(rth, stream)
                                ).unwrap();
                                *trx = Some(pytransport.clone());
                                pytransport
                            }))
                        },
                        _ => error_proto!()
                    }
                },
                _ => error_proto!()
            }
        })
    }
}

#[inline(always)]
fn message_into_py(message: Message) -> PyResult<PyObject> {
    match message {
        Message::Binary(message) => {
            Ok(Python::with_gil(|py| {
                WebsocketInboundBytesMessage::new(
                    PyBytes::new(py, &message).into()
                ).into_py(py)
            }))
        },
        Message::Text(message) => {
            Ok(Python::with_gil(|py| {
                WebsocketInboundTextMessage::new(
                    PyString::new(py, &message).into()
                ).into_py(py)
            }))
        },
        Message::Close(_) => {
            Ok(Python::with_gil(|py| {
                WebsocketInboundCloseMessage::new().into_py(py)
            }))
        }
        v => {
            log::warn!("Unsupported websocket message received {:?}", v);
            error_proto!()
        }
    }
}
