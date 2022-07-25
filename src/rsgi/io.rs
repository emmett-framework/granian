use bytes::Buf;
use futures::{sink::SinkExt, stream::StreamExt};
use hyper::{Body, Request};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::sync::Arc;
use tokio_tungstenite::WebSocketStream;
use tokio::sync::Mutex;
use tungstenite::Message;

use crate::{
    runtime::{RuntimeRef, future_into_py},
    ws::{HyperWebsocket, UpgradeData}
};
use super::errors::{RSGIProtocolError, error_proto};


#[pyclass(module="granian._granian")]
pub(crate) struct RSGIHTTPProtocol {
    rt: RuntimeRef,
    request: Arc<Mutex<Request<Body>>>
}

impl RSGIHTTPProtocol {
    pub fn new(rt: RuntimeRef, request: Request<Body>) -> Self {
        Self {
            rt: rt,
            request: Arc::new(Mutex::new(request))
        }
    }
}

#[pymethods]
impl RSGIHTTPProtocol {
    fn __call__<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let req_ref = self.request.clone();
        future_into_py(self.rt.clone(), py, async move {
            let mut req = req_ref.lock().await;
            let mut body = hyper::body::to_bytes(&mut *req).await.unwrap();
            Ok(Python::with_gil(|py| {
                // PyBytes::new(py, &body.to_vec());
                PyBytes::new_with(py, body.len(), |bytes: &mut [u8]| {
                    body.copy_to_slice(bytes);
                    Ok(())
                }).unwrap().as_ref().to_object(py)
            }))
        })
    }
}

#[pyclass(module="granian._granian")]
pub(crate) struct RSGIWebsocketTransport {
    rt: RuntimeRef,
    transport: Arc<Mutex<WebSocketStream<hyper::upgrade::Upgraded>>>
}

impl RSGIWebsocketTransport {
    pub fn new(
        rt: RuntimeRef,
        transport: WebSocketStream<hyper::upgrade::Upgraded>
    ) -> Self {
        Self { rt: rt, transport: Arc::new(Mutex::new(transport)) }
    }
}

#[pymethods]
impl RSGIWebsocketTransport {
    fn receive<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let transport = self.transport.clone();
        future_into_py(self.rt.clone(), py, async move {
            let mut stream = transport.lock().await;
            match stream.next().await {
                Some(recv) => {
                    match recv {
                        Ok(message) => message_into_py(message),
                        _ => error_proto!()
                    }
                },
                _ => error_proto!()
            }
        })
    }

    fn send_bytes<'p>(&self, py: Python<'p>, data: Vec<u8>) -> PyResult<&'p PyAny> {
        let transport = self.transport.clone();
        future_into_py(self.rt.clone(), py, async move {
            let mut stream = transport.lock().await;
            match stream.send(Message::Binary(data)).await {
                Ok(_) => Ok(()),
                _ => error_proto!()
            }
        })
    }

    fn send_str<'p>(&self, py: Python<'p>, data: String) -> PyResult<&'p PyAny> {
        let transport = self.transport.clone();
        future_into_py(self.rt.clone(), py, async move {
            let mut stream = transport.lock().await;
            match stream.send(Message::Text(data)).await {
                Ok(_) => Ok(()),
                _ => error_proto!()
            }
        })
    }
}

#[pyclass(module="granian._granian")]
pub(crate) struct RSGIWebsocketProtocol {
    rt: RuntimeRef,
    websocket: Arc<Mutex<HyperWebsocket>>,
    upgrade: Option<UpgradeData>,
    status: i32
}

impl RSGIWebsocketProtocol {
    pub fn new(
        rt: RuntimeRef,
        websocket: HyperWebsocket,
        upgrade: UpgradeData
    ) -> Self {
        Self {
            rt: rt,
            websocket: Arc::new(Mutex::new(websocket)),
            upgrade: Some(upgrade),
            status: 0
        }
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
    data: Vec<u8>
}

impl WebsocketInboundBytesMessage {
    pub fn new(data: Vec<u8>) -> Self {
        Self { kind: WebsocketMessageType::Bytes as usize, data: data }
    }
}

#[pyclass]
struct WebsocketInboundTextMessage {
    #[pyo3(get)]
    kind: usize,
    #[pyo3(get)]
    data: String
}

impl WebsocketInboundTextMessage {
    pub fn new(data: String) -> Self {
        Self { kind: WebsocketMessageType::Text as usize, data: data }
    }
}

#[pymethods]
impl RSGIWebsocketProtocol {
    #[args(status="None")]
    fn close(&mut self, status: Option<i32>) -> PyResult<(i32, bool)> {
        let consumed = match &self.upgrade {
            Some(_) => false,
            _ => true
        };
        let status = match status {
            Some(v) => v,
            _ => 0
        };
        self.status = status;
        Ok((status, consumed))
    }

    fn accept<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let rth = self.rt.clone();
        let mut upgrade = self.upgrade.take().unwrap();
        let transport = self.websocket.clone();
        future_into_py(self.rt.clone(), py, async move {
            let mut ws = transport.lock().await;
            match upgrade.send().await {
                Ok(_) => {
                    match (&mut *ws).await {
                        Ok(stream) => {
                            Ok(Python::with_gil(|py| {
                                RSGIWebsocketTransport::new(rth, stream).into_py(py)
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

#[inline]
fn message_into_py(message: Message) -> PyResult<PyObject> {
    match message {
        Message::Binary(message) => {
            Ok(Python::with_gil(|py| {
                WebsocketInboundBytesMessage::new(
                    message.to_vec()
                ).into_py(py)
            }))
        },
        Message::Text(message) => {
            Ok(Python::with_gil(|py| {
                WebsocketInboundTextMessage::new(message).into_py(py)
            }))
        },
        Message::Close(_) => {
            Ok(Python::with_gil(|py| {
                WebsocketInboundCloseMessage::new().into_py(py)
            }))
        }
        _ => error_proto!()
    }
}
