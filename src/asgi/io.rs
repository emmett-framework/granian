use futures::{sink::SinkExt, stream::{SplitSink, SplitStream, StreamExt}};
use hyper::{
    Body,
    Request,
    Response,
    header::{HeaderName, HeaderValue, HeaderMap, SERVER as HK_SERVER}
};
use pyo3::prelude::*;
use pyo3::pyclass::PyClass;
use pyo3::types::{PyBytes, PyDict};
use std::sync::Arc;
use tokio_tungstenite::WebSocketStream;
use tokio::sync::{Mutex, oneshot};
use tungstenite::Message;

use crate::{
    http::HV_SERVER,
    runtime::{RuntimeRef, future_into_py},
    ws::{HyperWebsocket, UpgradeData}
};
use super::{
    errors::{UnsupportedASGIMessage, error_flow, error_message},
    types::ASGIMessageType
};


const EMPTY_BYTES: Vec<u8> = Vec::new();
const EMPTY_STRING: String = String::new();

pub(crate) trait ASGIProtocol: PyClass {
    fn _recv<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny>;
    fn _send<'p>(&mut self, py: Python<'p>, data: &'p PyDict) -> PyResult<&'p PyAny>;
}

#[pyclass(module="granian._granian")]
pub(crate) struct ASGIHTTPProtocol {
    rt: RuntimeRef,
    tx: Option<oneshot::Sender<Response<Body>>>,
    request: Arc<Mutex<Request<Body>>>,
    response_inited: bool,
    response_built: bool,
    response_status: i16,
    response_headers: HeaderMap,
    response_body: Vec<u8>
}

impl ASGIHTTPProtocol {
    pub fn new(
        rt: RuntimeRef,
        request: Request<Body>,
        tx: oneshot::Sender<Response<Body>>
    ) -> Self {
        Self {
            rt: rt,
            tx: Some(tx),
            request: Arc::new(Mutex::new(request)),
            response_inited: false,
            response_built: false,
            response_status: 0,
            response_headers: HeaderMap::new(),
            response_body: Vec::new()
        }
    }

    #[inline(always)]
    fn send_body(&mut self, body: &[u8], finish: bool) {
        self.response_body.extend_from_slice(body);
        if finish {
            if let Some(tx) = self.tx.take() {
                let mut res = Response::new(self.response_body.to_owned().into());
                *res.status_mut() = hyper::StatusCode::from_u16(
                    self.response_status as u16
                ).unwrap();
                *res.headers_mut() = self.response_headers.to_owned();
                let _ = tx.send(res);
            }
            self.response_built = true;
        }
    }

    pub fn tx(&mut self) -> Option<oneshot::Sender<Response<Body>>> {
        return self.tx.take()
    }
}

#[pymethods]
impl ASGIHTTPProtocol {
    fn receive<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        self._recv(py)
    }

    fn send<'p>(&mut self, py: Python<'p>, data: &'p PyDict) -> PyResult<&'p PyAny> {
        self._send(py, data)
    }
}

#[pyclass(module="granian._granian")]
pub(crate) struct ASGIWebsocketProtocol {
    rt: RuntimeRef,
    tx: Option<oneshot::Sender<bool>>,
    websocket: Option<HyperWebsocket>,
    upgrade: Option<UpgradeData>,
    ws_tx: Arc<Mutex<Option<SplitSink<WebSocketStream<hyper::upgrade::Upgraded>, Message>>>>,
    ws_rx: Arc<Mutex<Option<SplitStream<WebSocketStream<hyper::upgrade::Upgraded>>>>>,
    accepted_rx: Option<oneshot::Receiver<bool>>,
    accepted_tx: Option<oneshot::Sender<bool>>,
    closed: bool
}

impl ASGIWebsocketProtocol {
    pub fn new(
        rt: RuntimeRef,
        tx: oneshot::Sender<bool>,
        websocket: HyperWebsocket,
        upgrade: UpgradeData
    ) -> Self {
        let (accepted_tx, accepted_rx) = oneshot::channel();

        Self {
            rt: rt,
            tx: Some(tx),
            websocket: Some(websocket),
            upgrade: Some(upgrade),
            ws_tx: Arc::new(Mutex::new(None)),
            ws_rx: Arc::new(Mutex::new(None)),
            accepted_rx: Some(accepted_rx),
            accepted_tx: Some(accepted_tx),
            closed: false
        }
    }

    #[inline(always)]
    fn accept<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let mut upgrade = self.upgrade.take().unwrap();
        let websocket = self.websocket.take().unwrap();
        let accepted = self.accepted_tx.take().unwrap();
        let tx = self.ws_tx.clone();
        let rx = self.ws_rx.clone();
        future_into_py(self.rt.clone(), py, async move {
            if let Ok(_) = upgrade.send().await {
                if let Ok(stream) = websocket.await {
                    let mut wtx = tx.lock().await;
                    let mut wrx = rx.lock().await;
                    let (tx, rx) = stream.split();
                    *wtx = Some(tx);
                    *wrx = Some(rx);
                    return match accepted.send(true) {
                        Ok(_) => Ok(()),
                        _ => error_flow!()
                    }
                }
            }
            error_flow!()
        })
    }

    #[inline(always)]
    fn send_message<'p>(
        &self,
        py: Python<'p>,
        data: &'p PyDict
    ) -> PyResult<&'p PyAny> {
        let transport = self.ws_tx.clone();
        let closed = self.closed.clone();
        let message = adapt_ws_message(data);
        future_into_py(self.rt.clone(), py, async move {
            if !closed {
                if let Some(ws) = &mut *(transport.lock().await) {
                    if let Ok(_) = ws.send(message).await {
                        return Ok(())
                    }
                };
            };
            error_flow!()
        })
    }

    fn consumed(&self) -> bool {
        match &self.upgrade {
            Some(_) => false,
            _ => true
        }
    }

    pub fn tx(&mut self) -> (Option<oneshot::Sender<bool>>, bool) {
        (self.tx.take(), self.consumed())
    }
}

macro_rules! empty_future {
    ($rt:expr, $py:expr) => {
        future_into_py($rt, $py, async move {
            Ok(())
        })
    };
}

#[pymethods]
impl ASGIWebsocketProtocol {
    fn receive<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        self._recv(py)
    }

    fn send<'p>(&mut self, py: Python<'p>, data: &'p PyDict) -> PyResult<&'p PyAny> {
        self._send(py, data)
    }
}

impl ASGIProtocol for ASGIHTTPProtocol {
    #[inline(always)]
    fn _recv<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let transport = self.request.clone();
        future_into_py(self.rt.clone(), py, async move {
            let mut req = transport.lock().await;
            let body = hyper::body::to_bytes(&mut *req).await.unwrap();
            Python::with_gil(|py| {
                let dict = PyDict::new(py);
                dict.set_item("type", "http.request")?;
                dict.set_item("body", &body.to_vec())?;
                dict.set_item("more_body", false)?;
                Ok(dict.to_object(py))
            })
        })
    }

    #[inline(always)]
    fn _send<'p>(&mut self, py: Python<'p>, data: &'p PyDict) -> PyResult<&'p PyAny> {
        match adapt_message_type(data) {
            Ok(ASGIMessageType::HTTPStart) => {
                match self.response_inited {
                    false => {
                        self.response_status = adapt_status_code(data).unwrap();
                        self.response_headers = adapt_headers(data);
                        self.response_inited = true;
                        empty_future!(self.rt.clone(), py)
                    },
                    _ => error_flow!()
                }
            },
            Ok(ASGIMessageType::HTTPBody) => {
                match (self.response_inited, self.response_built) {
                    (true, false) => {
                        let (body, more) = adapt_body(data);
                        self.send_body(&body[..], !more);
                        empty_future!(self.rt.clone(), py)
                    },
                    _ => error_flow!()
                }
            },
            Err(err) => Err(err.into()),
            _ => error_message!()
        }
    }
}

impl ASGIProtocol for ASGIWebsocketProtocol {
    #[inline(always)]
    fn _recv<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let transport = self.ws_rx.clone();
        let closed = self.closed.clone();
        let accepted = self.accepted_rx.take();
        future_into_py(self.rt.clone(), py, async move {
            match (accepted, closed) {
                (Some(accepted), false) => {
                    match accepted.await {
                        Ok(true) => {
                            return Python::with_gil(|py| {
                                let dict = PyDict::new(py);
                                dict.set_item("type", "websocket.connect")?;
                                Ok(dict.to_object(py))
                            })
                        },
                        _ => {
                            return error_flow!()
                        }
                    }
                },
                (None, false) => {},
                _ => {
                    return error_flow!()
                }
            }
            if let Some(ws) = &mut *(transport.lock().await) {
                if let Some(recv) = ws.next().await {
                    if let Ok(message) = recv {
                        return match message {
                            Message::Binary(message) => {
                                Python::with_gil(|py| {
                                    let dict = PyDict::new(py);
                                    dict.set_item("type", "websocket.receive")?;
                                    dict.set_item("bytes", PyBytes::new(py, &message[..]))?;
                                    Ok(dict.to_object(py))
                                })
                            },
                            Message::Text(message) => {
                                Python::with_gil(|py| {
                                    let dict = PyDict::new(py);
                                    dict.set_item("type", "websocket.receive")?;
                                    dict.set_item("text", message)?;
                                    Ok(dict.to_object(py))
                                })
                            },
                            Message::Close(_) => {
                                Python::with_gil(|py| {
                                    let dict = PyDict::new(py);
                                    dict.set_item("type", "websocket.disconnect")?;
                                    Ok(dict.to_object(py))
                                })
                            },
                            _ => error_flow!()
                        }
                    }
                }
            };
            error_flow!()
        })
    }

    #[inline(always)]
    fn _send<'p>(&mut self, py: Python<'p>, data: &'p PyDict) -> PyResult<&'p PyAny> {
        match adapt_message_type(data) {
            Ok(ASGIMessageType::WSAccept) => {
                self.accept(py)
            },
            Ok(ASGIMessageType::WSClose) => {
                self.closed = true;
                empty_future!(self.rt.clone(), py)
            },
            Ok(ASGIMessageType::WSMessage) => {
                self.send_message(py, data)
            },
            Err(err) => Err(err.into()),
            _ => error_message!()
        }
    }
}

#[inline(never)]
fn adapt_message_type(
    message: &PyDict
) -> Result<ASGIMessageType, UnsupportedASGIMessage> {
    match message.get_item("type") {
        Some(item) => {
            let message_type: &str = item.extract()?;
            match message_type {
                "http.response.start" => Ok(ASGIMessageType::HTTPStart),
                "http.response.body" => Ok(ASGIMessageType::HTTPBody),
                "websocket.accept" => Ok(ASGIMessageType::WSAccept),
                "websocket.close" => Ok(ASGIMessageType::WSClose),
                "websocket.send" => Ok(ASGIMessageType::WSMessage),
                _ => error_message!()
            }
        },
        _ => error_message!()
    }
}

#[inline(always)]
fn adapt_status_code(message: &PyDict) -> Result<i16, UnsupportedASGIMessage> {
    match message.get_item("status") {
        Some(item) => {
            Ok(item.extract()?)
        },
        _ => error_message!()
    }
}

#[inline(always)]
fn adapt_headers(message: &PyDict) -> HeaderMap {
    let mut ret = HeaderMap::new();
    ret.insert(HK_SERVER, HV_SERVER);
    match message.get_item("headers") {
        Some(item) => {
            let accum: Vec<Vec<&[u8]>> = item.extract().unwrap_or(Vec::new());
            for tup in accum.iter() {
                match (
                    HeaderName::from_bytes(tup[0]),
                    HeaderValue::from_bytes(tup[1])
                    ) {
                    (Ok(key), Ok(val)) => { ret.insert(key, val); },
                    _ => {}
                }
            };
            ret
        },
        _ => ret
    }
}

#[inline(always)]
fn adapt_body(message: &PyDict) -> (Vec<u8>, bool) {
    let body = match message.get_item("body") {
        Some(item) => {
            item.extract().unwrap_or(EMPTY_BYTES)
        },
        _ => EMPTY_BYTES
    };
    let more = match message.get_item("more_body") {
        Some(item) => {
            item.extract().unwrap_or(false)
        },
        _ => false
    };
    (body, more)
}

#[inline(always)]
fn adapt_ws_message(message: &PyDict) -> Message {
    match message.contains("bytes") {
        Ok(true) => {
            let data = match message.get_item("bytes") {
                Some(item) => {
                    item.extract().unwrap_or(EMPTY_BYTES)
                },
                _ => EMPTY_BYTES
            };
            Message::Binary(data)
        },
        Ok(false) => {
            let data = match message.get_item("text") {
                Some(item) => {
                    item.extract().unwrap_or(EMPTY_STRING)
                },
                _ => EMPTY_STRING
            };
            Message::Text(data)
        },
        _ => Message::Binary(EMPTY_BYTES)
    }
}
