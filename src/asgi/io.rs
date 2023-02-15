use futures::{sink::SinkExt, stream::{SplitSink, SplitStream, StreamExt}};
use hyper::{
    Body,
    Request,
    Response,
    header::{HeaderName, HeaderValue, HeaderMap, SERVER as HK_SERVER}
};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use std::sync::Arc;
use tokio_tungstenite::WebSocketStream;
use tokio::sync::{Mutex, oneshot};
use tungstenite::Message;

use crate::{
    http::HV_SERVER,
    runtime::{RuntimeRef, future_into_py_iter, future_into_py_futlike},
    ws::{HyperWebsocket, UpgradeData}
};
use super::{
    errors::{UnsupportedASGIMessage, error_flow, error_message},
    types::ASGIMessageType
};


const EMPTY_BYTES: Vec<u8> = Vec::new();
const EMPTY_STRING: String = String::new();

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
        self.tx.take()
    }
}

#[pymethods]
impl ASGIHTTPProtocol {
    fn receive<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let transport = self.request.clone();
        future_into_py_iter(self.rt.clone(), py, async move {
            let mut req = transport.lock().await;
            let body = hyper::body::to_bytes(&mut *req).await.unwrap();
            Python::with_gil(|py| {
                let dict = PyDict::new(py);
                dict.set_item(
                    pyo3::intern!(py, "type"),
                    pyo3::intern!(py, "http.request")
                )?;
                dict.set_item(pyo3::intern!(py, "body"), PyBytes::new(py, &body[..]))?;
                dict.set_item(pyo3::intern!(py, "more_body"), false)?;
                Ok(dict.to_object(py))
            })
        })
    }

    fn send(&mut self, data: &PyDict) -> PyResult<()> {
        match adapt_message_type(data) {
            Ok(ASGIMessageType::HTTPStart) => {
                match self.response_inited {
                    false => {
                        self.response_status = adapt_status_code(data).unwrap();
                        self.response_headers = adapt_headers(data);
                        self.response_inited = true;
                        Ok(())
                    },
                    _ => error_flow!()
                }
            },
            Ok(ASGIMessageType::HTTPBody) => {
                match (self.response_inited, self.response_built) {
                    (true, false) => {
                        let (body, more) = adapt_body(data);
                        self.send_body(&body[..], !more);
                        Ok(())
                    },
                    _ => error_flow!()
                }
            },
            Err(err) => Err(err.into()),
            _ => error_message!()
        }
    }

    fn response_start(&mut self, status: i16, pyheaders: Vec<Vec<&[u8]>>) -> PyResult<()> {
        let mut headers = HeaderMap::new();
        headers.insert(HK_SERVER, HV_SERVER);
        self.response_status = status;
        for tup in pyheaders.iter() {
            match (
                HeaderName::from_bytes(tup[0]),
                HeaderValue::from_bytes(tup[1])
            ) {
                (Ok(key), Ok(val)) => {
                    headers.insert(key, val);
                },
                _ => {
                    return error_flow!()
                }
            }
        };
        self.response_headers = headers;
        self.response_inited = true;
        Ok(())
    }

    fn response_body(&mut self, body: &[u8], has_more: bool) -> PyResult<()> {
        match (self.response_inited, self.response_built) {
            (true, false) => {
                self.send_body(body, !has_more);
                Ok(())
            },
            _ => error_flow!()
        }
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
    accepted: Arc<Mutex<bool>>,
    closed: bool
}

impl ASGIWebsocketProtocol {
    pub fn new(
        rt: RuntimeRef,
        tx: oneshot::Sender<bool>,
        websocket: HyperWebsocket,
        upgrade: UpgradeData
    ) -> Self {
        Self {
            rt: rt,
            tx: Some(tx),
            websocket: Some(websocket),
            upgrade: Some(upgrade),
            ws_tx: Arc::new(Mutex::new(None)),
            ws_rx: Arc::new(Mutex::new(None)),
            accepted: Arc::new(Mutex::new(false)),
            closed: false
        }
    }

    #[inline(always)]
    fn accept<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let mut upgrade = self.upgrade.take().unwrap();
        let websocket = self.websocket.take().unwrap();
        let accepted = self.accepted.clone();
        let tx = self.ws_tx.clone();
        let rx = self.ws_rx.clone();
        future_into_py_iter(self.rt.clone(), py, async move {
            if let Ok(_) = upgrade.send().await {
                if let Ok(stream) = websocket.await {
                    let mut wtx = tx.lock().await;
                    let mut wrx = rx.lock().await;
                    let mut accepted = accepted.lock().await;
                    let (tx, rx) = stream.split();
                    *wtx = Some(tx);
                    *wrx = Some(rx);
                    *accepted = true;
                    return Ok(())
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
        let message = ws_message_into_rs(data);
        future_into_py_iter(self.rt.clone(), py, async move {
            if !closed {
                if let Ok(message) = message {
                    if let Some(ws) = &mut *(transport.lock().await) {
                        if let Ok(_) = ws.send(message).await {
                            return Ok(())
                        }
                    };
                };
            };
            error_flow!()
        })
    }

    #[inline(always)]
    fn close<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let transport = self.ws_tx.clone();
        let closed = self.closed.clone();
        future_into_py_iter(self.rt.clone(), py, async move {
            if !closed {
                if let Some(ws) = &mut *(transport.lock().await) {
                    if let Ok(_) = ws.close().await {
                        return Ok(())
                    }
                }
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

#[pymethods]
impl ASGIWebsocketProtocol {
    fn receive<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let transport = self.ws_rx.clone();
        let accepted = self.accepted.clone();
        let closed = self.closed.clone();
        future_into_py_futlike(self.rt.clone(), py, async move {
            let accepted = accepted.lock().await;
            match (*accepted, closed) {
                (false, false) => {
                    return Python::with_gil(|py| {
                        let dict = PyDict::new(py);
                        dict.set_item(
                            pyo3::intern!(py, "type"),
                            pyo3::intern!(py, "websocket.connect")
                        )?;
                        Ok(dict.to_object(py))
                    })
                },
                (true, false) => {},
                _ => {
                    return error_flow!()
                }
            }
            if let Some(ws) = &mut *(transport.lock().await) {
                loop {
                    match ws.next().await {
                        Some(recv) => {
                            match recv {
                                Ok(Message::Ping(_)) => {
                                    continue
                                },
                                Ok(message) => {
                                    return ws_message_into_py(message)
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
            }
            error_flow!()
        })
    }

    fn send<'p>(&mut self, py: Python<'p>, data: &'p PyDict) -> PyResult<&'p PyAny> {
        match adapt_message_type(data) {
            Ok(ASGIMessageType::WSAccept) => {
                self.accept(py)
            },
            Ok(ASGIMessageType::WSClose) => {
                self.close(py)
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
                    (Ok(key), Ok(val)) => { ret.append(key, val); },
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
fn ws_message_into_rs(message: &PyDict) -> PyResult<Message> {
    match (message.get_item("bytes"), message.get_item("text")) {
        (Some(item), None) => {
            Ok(Message::Binary(item.extract().unwrap_or(EMPTY_BYTES)))
        },
        (None, Some(item)) => {
            Ok(Message::Text(item.extract().unwrap_or(EMPTY_STRING)))
        },
        (Some(itemb), Some(itemt)) => {
            match (itemb.extract().unwrap_or(None), itemt.extract().unwrap_or(None)) {
                (Some(msgb), None) => {
                    Ok(Message::Binary(msgb))
                },
                (None, Some(msgt)) => {
                    Ok(Message::Text(msgt))
                },
                _ => error_flow!()
            }
        },
        _ => {
            error_flow!()
        }
    }
}

#[inline(always)]
fn ws_message_into_py(message: Message) -> PyResult<PyObject> {
    match message {
        Message::Binary(message) => {
            Python::with_gil(|py| {
                let dict = PyDict::new(py);
                dict.set_item(
                    pyo3::intern!(py, "type"),
                    pyo3::intern!(py, "websocket.receive")
                )?;
                dict.set_item(
                    pyo3::intern!(py, "bytes"),
                    PyBytes::new(py, &message[..])
                )?;
                Ok(dict.to_object(py))
            })
        },
        Message::Text(message) => {
            Python::with_gil(|py| {
                let dict = PyDict::new(py);
                dict.set_item(
                    pyo3::intern!(py, "type"),
                    pyo3::intern!(py, "websocket.receive")
                )?;
                dict.set_item(pyo3::intern!(py, "text"), message)?;
                Ok(dict.to_object(py))
            })
        },
        Message::Close(_) => {
            Python::with_gil(|py| {
                let dict = PyDict::new(py);
                dict.set_item(
                    pyo3::intern!(py, "type"),
                    pyo3::intern!(py, "websocket.disconnect")
                )?;
                Ok(dict.to_object(py))
            })
        },
        v => {
            log::warn!("Unsupported websocket message received {:?}", v);
            error_flow!()
        }
    }
}
