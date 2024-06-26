use anyhow::Result;
use futures::{sink::SinkExt, StreamExt, TryStreamExt};
use http_body_util::BodyExt;
use hyper::{
    body,
    header::{HeaderMap, HeaderName, HeaderValue, SERVER as HK_SERVER},
    Response, StatusCode,
};
use pyo3::{
    prelude::*,
    pybacked::PyBackedBytes,
    types::{PyBytes, PyDict},
};
use std::{
    borrow::Cow,
    sync::{atomic, Arc, Mutex},
};
use tokio::{
    fs::File,
    sync::{mpsc, oneshot, Mutex as AsyncMutex},
};
use tokio_tungstenite::tungstenite::Message;
use tokio_util::io::ReaderStream;

use super::{
    errors::{error_flow, error_message, UnsupportedASGIMessage},
    types::ASGIMessageType,
};
use crate::{
    conversion::BytesToPy,
    http::{response_404, HTTPResponse, HTTPResponseBody, HV_SERVER},
    runtime::{empty_future_into_py, future_into_py_futlike, future_into_py_iter, Runtime, RuntimeRef},
    ws::{HyperWebsocket, UpgradeData, WSRxStream, WSTxStream},
};

const EMPTY_BYTES: Cow<[u8]> = Cow::Borrowed(b"");
const EMPTY_STRING: String = String::new();
static WS_SUBPROTO_HNAME: &str = "Sec-WebSocket-Protocol";

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct ASGIHTTPProtocol {
    rt: RuntimeRef,
    tx: Mutex<Option<oneshot::Sender<HTTPResponse>>>,
    request_body: Arc<AsyncMutex<http_body_util::BodyStream<body::Incoming>>>,
    response_started: atomic::AtomicBool,
    response_chunked: atomic::AtomicBool,
    response_intent: Mutex<Option<(u16, HeaderMap)>>,
    body_tx: Mutex<Option<mpsc::Sender<Result<body::Bytes, anyhow::Error>>>>,
    flow_rx_exhausted: Arc<atomic::AtomicBool>,
    flow_tx_waiter: Arc<tokio::sync::Notify>,
    sent_response_code: Arc<atomic::AtomicU16>,
}

impl ASGIHTTPProtocol {
    pub fn new(rt: RuntimeRef, body: hyper::body::Incoming, tx: oneshot::Sender<HTTPResponse>) -> Self {
        Self {
            rt,
            tx: Mutex::new(Some(tx)),
            request_body: Arc::new(AsyncMutex::new(http_body_util::BodyStream::new(body))),
            response_started: false.into(),
            response_chunked: false.into(),
            response_intent: Mutex::new(None),
            body_tx: Mutex::new(None),
            flow_rx_exhausted: Arc::new(atomic::AtomicBool::new(false)),
            flow_tx_waiter: Arc::new(tokio::sync::Notify::new()),
            sent_response_code: Arc::new(atomic::AtomicU16::new(500)),
        }
    }

    #[inline(always)]
    fn send_response(&self, status: u16, headers: HeaderMap<HeaderValue>, body: HTTPResponseBody) {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            let mut res = Response::new(body);
            *res.status_mut() = hyper::StatusCode::from_u16(status).unwrap();
            *res.headers_mut() = headers;
            let _ = tx.send(res);
            self.sent_response_code.store(status, atomic::Ordering::Relaxed);
        }
    }

    #[inline]
    fn send_body<'p>(
        &self,
        py: Python<'p>,
        tx: mpsc::Sender<Result<body::Bytes, anyhow::Error>>,
        body: Box<[u8]>,
        close: bool,
    ) -> PyResult<Bound<'p, PyAny>> {
        let flow_hld = self.flow_tx_waiter.clone();
        future_into_py_futlike(self.rt.clone(), py, async move {
            match tx.send(Ok(body.into())).await {
                Ok(()) => {
                    if close {
                        flow_hld.notify_one();
                    }
                }
                Err(err) => {
                    log::warn!("ASGI transport error: {:?}", err);
                    flow_hld.notify_one();
                }
            }
            Ok(())
        })
    }

    pub fn tx(&self) -> Option<oneshot::Sender<HTTPResponse>> {
        self.tx.lock().unwrap().take()
    }
}

#[pymethods]
impl ASGIHTTPProtocol {
    fn receive<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        if self.flow_rx_exhausted.load(atomic::Ordering::Relaxed) {
            let flow_hld = self.flow_tx_waiter.clone();
            return future_into_py_futlike(self.rt.clone(), py, async move {
                let () = flow_hld.notified().await;
                Python::with_gil(|py| {
                    let dict = PyDict::new_bound(py);
                    dict.set_item(pyo3::intern!(py, "type"), pyo3::intern!(py, "http.disconnect"))?;
                    Ok(dict.to_object(py))
                })
            });
        }

        let body_ref = self.request_body.clone();
        let flow_ref = self.flow_rx_exhausted.clone();
        let flow_hld = self.flow_tx_waiter.clone();
        future_into_py_iter(self.rt.clone(), py, async move {
            let mut bodym = body_ref.lock().await;
            let body = &mut *bodym;
            let mut more_body = false;
            let chunk = match body.next().await {
                Some(Ok(buf)) => {
                    more_body = true;
                    Ok(buf.into_data().unwrap_or_default())
                }
                Some(Err(err)) => Err(err),
                _ => Ok(body::Bytes::new()),
            };
            if !more_body {
                flow_ref.store(true, atomic::Ordering::Relaxed);
            }

            match chunk {
                Ok(data) => Python::with_gil(|py| {
                    let dict = PyDict::new_bound(py);
                    dict.set_item(pyo3::intern!(py, "type"), pyo3::intern!(py, "http.request"))?;
                    dict.set_item(pyo3::intern!(py, "body"), BytesToPy(data))?;
                    dict.set_item(pyo3::intern!(py, "more_body"), more_body)?;
                    Ok(dict.to_object(py))
                }),
                _ => {
                    flow_hld.notify_one();
                    Python::with_gil(|py| {
                        let dict = PyDict::new_bound(py);
                        dict.set_item(pyo3::intern!(py, "type"), pyo3::intern!(py, "http.disconnect"))?;
                        Ok(dict.to_object(py))
                    })
                }
            }
        })
    }

    fn send<'p>(&self, py: Python<'p>, data: &Bound<'p, PyDict>) -> PyResult<Bound<'p, PyAny>> {
        match adapt_message_type(py, data) {
            Ok(ASGIMessageType::HTTPStart(intent)) => match self.response_started.load(atomic::Ordering::Relaxed) {
                false => {
                    let mut response_intent = self.response_intent.lock().unwrap();
                    *response_intent = Some(intent);
                    self.response_started.store(true, atomic::Ordering::Relaxed);
                    empty_future_into_py(py)
                }
                true => error_flow!(),
            },
            Ok(ASGIMessageType::HTTPBody((body, more))) => {
                match (
                    self.response_started.load(atomic::Ordering::Relaxed),
                    more,
                    self.response_chunked.load(atomic::Ordering::Relaxed),
                ) {
                    (true, false, false) => {
                        let (status, headers) = self.response_intent.lock().unwrap().take().unwrap();
                        self.send_response(
                            status,
                            headers,
                            http_body_util::Full::new(body::Bytes::from(body))
                                .map_err(|e| match e {})
                                .boxed(),
                        );
                        self.flow_tx_waiter.notify_one();
                        empty_future_into_py(py)
                    }
                    (true, true, false) => {
                        self.response_chunked.store(true, atomic::Ordering::Relaxed);
                        let (status, headers) = self.response_intent.lock().unwrap().take().unwrap();
                        let (body_tx, body_rx) = mpsc::channel::<Result<body::Bytes, anyhow::Error>>(1);
                        let body_stream = http_body_util::StreamBody::new(
                            tokio_stream::wrappers::ReceiverStream::new(body_rx).map_ok(body::Frame::data),
                        );
                        *self.body_tx.lock().unwrap() = Some(body_tx.clone());
                        self.send_response(status, headers, BodyExt::boxed(body_stream));
                        self.send_body(py, body_tx, body, false)
                    }
                    (true, true, true) => match &*self.body_tx.lock().unwrap() {
                        Some(tx) => {
                            let tx = tx.clone();
                            self.send_body(py, tx, body, false)
                        }
                        _ => error_flow!(),
                    },
                    (true, false, true) => match self.body_tx.lock().unwrap().take() {
                        Some(tx) => match body.is_empty() {
                            false => self.send_body(py, tx, body, true),
                            true => {
                                self.flow_tx_waiter.notify_one();
                                empty_future_into_py(py)
                            }
                        },
                        _ => error_flow!(),
                    },
                    _ => error_flow!(),
                }
            }
            Ok(ASGIMessageType::HTTPFile(file_path)) => match (
                self.response_started.load(atomic::Ordering::Relaxed),
                self.tx.lock().unwrap().take(),
            ) {
                (true, Some(tx)) => {
                    let sent_response = self.sent_response_code.clone();
                    let (status, headers) = self.response_intent.lock().unwrap().take().unwrap();
                    self.rt.spawn(async move {
                        let res = match File::open(&file_path).await {
                            Ok(file) => {
                                let stream = ReaderStream::new(file);
                                let stream_body = http_body_util::StreamBody::new(stream.map_ok(body::Frame::data));
                                let mut res =
                                    Response::new(BodyExt::map_err(stream_body, std::convert::Into::into).boxed());
                                *res.status_mut() = StatusCode::from_u16(status).unwrap();
                                *res.headers_mut() = headers;
                                sent_response.store(status, atomic::Ordering::Relaxed);
                                res
                            }
                            Err(_) => {
                                log::info!("Cannot open file {}", &file_path);
                                sent_response.store(404, atomic::Ordering::Relaxed);
                                response_404()
                            }
                        };
                        let _ = tx.send(res);
                    });
                    empty_future_into_py(py)
                }
                _ => error_flow!(),
            },
            Err(err) => Err(err.into()),
            _ => error_message!(),
        }
    }

    #[getter(sent_response_code)]
    fn get_sent_response_code(&self) -> u16 {
        self.sent_response_code.load(atomic::Ordering::Relaxed)
    }
}

pub(crate) struct WebsocketDetachedTransport {
    pub consumed: bool,
    rx: Option<WSRxStream>,
    tx: Option<WSTxStream>,
}

impl WebsocketDetachedTransport {
    pub fn new(consumed: bool, rx: Option<WSRxStream>, tx: Option<WSTxStream>) -> Self {
        Self { consumed, rx, tx }
    }

    pub async fn close(&mut self) {
        if let Some(mut tx) = self.tx.take() {
            if let Err(err) = tx.close().await {
                log::info!("Failed to close websocket with error {:?}", err);
            }
        }
        drop(self.rx.take());
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct ASGIWebsocketProtocol {
    rt: RuntimeRef,
    tx: Mutex<Option<oneshot::Sender<WebsocketDetachedTransport>>>,
    websocket: Mutex<Option<HyperWebsocket>>,
    upgrade: Mutex<Option<UpgradeData>>,
    ws_rx: Arc<AsyncMutex<Option<WSRxStream>>>,
    ws_tx: Arc<AsyncMutex<Option<WSTxStream>>>,
    accepted: Arc<atomic::AtomicBool>,
    closed: Arc<atomic::AtomicBool>,
}

impl ASGIWebsocketProtocol {
    pub fn new(
        rt: RuntimeRef,
        tx: oneshot::Sender<WebsocketDetachedTransport>,
        websocket: HyperWebsocket,
        upgrade: UpgradeData,
    ) -> Self {
        Self {
            rt,
            tx: Mutex::new(Some(tx)),
            websocket: Mutex::new(Some(websocket)),
            upgrade: Mutex::new(Some(upgrade)),
            ws_rx: Arc::new(AsyncMutex::new(None)),
            ws_tx: Arc::new(AsyncMutex::new(None)),
            accepted: Arc::new(false.into()),
            closed: Arc::new(false.into()),
        }
    }

    #[inline(always)]
    fn accept<'p>(&self, py: Python<'p>, subproto: Option<String>) -> PyResult<Bound<'p, PyAny>> {
        let upgrade = self.upgrade.lock().unwrap().take();
        let websocket = self.websocket.lock().unwrap().take();
        let accepted = self.accepted.clone();
        let rx = self.ws_rx.clone();
        let tx = self.ws_tx.clone();

        future_into_py_iter(self.rt.clone(), py, async move {
            if let Some(mut upgrade) = upgrade {
                let upgrade_headers = match subproto {
                    Some(v) => vec![(WS_SUBPROTO_HNAME.to_string(), v)],
                    _ => vec![],
                };
                if (upgrade.send(Some(upgrade_headers)).await).is_ok() {
                    if let Some(websocket) = websocket {
                        if let Ok(stream) = websocket.await {
                            let mut wtx = tx.lock().await;
                            let mut wrx = rx.lock().await;
                            let (tx, rx) = stream.split();
                            *wtx = Some(tx);
                            *wrx = Some(rx);
                            accepted.store(true, atomic::Ordering::Relaxed);
                            return Ok(());
                        }
                    }
                }
            }
            error_flow!()
        })
    }

    #[inline(always)]
    fn send_message<'p>(&self, py: Python<'p>, data: Message) -> PyResult<Bound<'p, PyAny>> {
        let transport = self.ws_tx.clone();
        let closed = self.closed.clone();

        future_into_py_futlike(self.rt.clone(), py, async move {
            if let Some(ws) = &mut *(transport.lock().await) {
                match ws.send(data).await {
                    Ok(()) => return Ok(()),
                    _ => {
                        if closed.load(atomic::Ordering::Relaxed) {
                            log::info!("Attempted to write to a closed websocket");
                            return Ok(());
                        }
                    }
                };
            };
            error_flow!()
        })
    }

    #[inline(always)]
    fn close<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let closed = self.closed.clone();
        let ws_rx = self.ws_rx.clone();
        let ws_tx = self.ws_tx.clone();

        future_into_py_iter(self.rt.clone(), py, async move {
            match ws_tx.lock().await.take() {
                Some(tx) => {
                    closed.store(true, atomic::Ordering::Relaxed);
                    WebsocketDetachedTransport::new(true, ws_rx.lock().await.take(), Some(tx))
                        .close()
                        .await;
                    Ok(())
                }
                _ => error_flow!(),
            }
        })
    }

    fn consumed(&self) -> bool {
        self.upgrade.lock().unwrap().is_none()
    }

    pub fn tx(
        &self,
    ) -> (
        Option<oneshot::Sender<WebsocketDetachedTransport>>,
        WebsocketDetachedTransport,
    ) {
        let mut ws_rx = self.ws_rx.blocking_lock();
        let mut ws_tx = self.ws_tx.blocking_lock();
        (
            self.tx.lock().unwrap().take(),
            WebsocketDetachedTransport::new(self.consumed(), ws_rx.take(), ws_tx.take()),
        )
    }
}

#[pymethods]
impl ASGIWebsocketProtocol {
    fn receive<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        let accepted = self.accepted.clone();
        let closed = self.closed.clone();
        let transport = self.ws_rx.clone();

        future_into_py_futlike(self.rt.clone(), py, async move {
            let accepted = accepted.load(atomic::Ordering::Relaxed);
            if !accepted {
                return Python::with_gil(|py| {
                    let dict = PyDict::new_bound(py);
                    dict.set_item(pyo3::intern!(py, "type"), pyo3::intern!(py, "websocket.connect"))?;
                    Ok(dict.to_object(py))
                });
            }

            if let Some(ws) = &mut *(transport.lock().await) {
                while let Some(recv) = ws.next().await {
                    match recv {
                        Ok(Message::Ping(_) | Message::Pong(_)) => continue,
                        Ok(message @ Message::Close(_)) => {
                            closed.store(true, atomic::Ordering::Relaxed);
                            return ws_message_into_py(message);
                        }
                        Ok(message) => return ws_message_into_py(message),
                        _ => break,
                    }
                }
            }
            error_flow!()
        })
    }

    fn send<'p>(&self, py: Python<'p>, data: &Bound<'p, PyDict>) -> PyResult<Bound<'p, PyAny>> {
        match adapt_message_type(py, data) {
            Ok(ASGIMessageType::WSAccept(subproto)) => self.accept(py, subproto),
            Ok(ASGIMessageType::WSClose) => self.close(py),
            Ok(ASGIMessageType::WSMessage(message)) => self.send_message(py, message),
            _ => future_into_py_iter::<_, _, PyErr>(self.rt.clone(), py, async { error_message!() }),
        }
    }
}

#[inline(never)]
fn adapt_message_type(py: Python, message: &Bound<PyDict>) -> Result<ASGIMessageType, UnsupportedASGIMessage> {
    match message.get_item(pyo3::intern!(py, "type")) {
        Ok(Some(item)) => {
            let message_type: &str = item.extract()?;
            match message_type {
                "http.response.start" => Ok(ASGIMessageType::HTTPStart((
                    adapt_status_code(py, message)?,
                    adapt_headers(py, message),
                ))),
                "http.response.body" => Ok(ASGIMessageType::HTTPBody(adapt_body(py, message))),
                "http.response.pathsend" => Ok(ASGIMessageType::HTTPFile(adapt_file(py, message)?)),
                "websocket.accept" => {
                    let subproto: Option<String> = match message.get_item(pyo3::intern!(py, "subprotocol")) {
                        Ok(Some(item)) => item.extract::<String>().map(Some).unwrap_or(None),
                        _ => None,
                    };
                    Ok(ASGIMessageType::WSAccept(subproto))
                }
                "websocket.close" => Ok(ASGIMessageType::WSClose),
                "websocket.send" => Ok(ASGIMessageType::WSMessage(ws_message_into_rs(py, message)?)),
                _ => error_message!(),
            }
        }
        _ => error_message!(),
    }
}

#[inline(always)]
fn adapt_status_code(py: Python, message: &Bound<PyDict>) -> Result<u16, UnsupportedASGIMessage> {
    match message.get_item(pyo3::intern!(py, "status"))? {
        Some(item) => Ok(item.extract()?),
        _ => error_message!(),
    }
}

#[inline(always)]
fn adapt_headers(py: Python, message: &Bound<PyDict>) -> HeaderMap {
    let mut ret = HeaderMap::new();
    ret.insert(HK_SERVER, HV_SERVER);
    match message.get_item(pyo3::intern!(py, "headers")) {
        Ok(Some(item)) => {
            let accum: Vec<Vec<PyBackedBytes>> = item.extract().unwrap_or(Vec::new());
            for tup in &accum {
                if let (Ok(key), Ok(val)) = (HeaderName::from_bytes(&tup[0]), HeaderValue::from_bytes(&tup[1])) {
                    ret.append(key, val);
                }
            }
            ret
        }
        _ => ret,
    }
}

#[inline(always)]
fn adapt_body(py: Python, message: &Bound<PyDict>) -> (Box<[u8]>, bool) {
    let body = message.get_item(pyo3::intern!(py, "body"));
    let body = match body {
        Ok(Some(ref item)) => item.extract().unwrap_or(EMPTY_BYTES),
        _ => EMPTY_BYTES,
    };
    let more = match message.get_item(pyo3::intern!(py, "more_body")) {
        Ok(Some(item)) => item.extract().unwrap_or(false),
        _ => false,
    };
    (body.into(), more)
}

#[inline(always)]
fn adapt_file(py: Python, message: &Bound<PyDict>) -> PyResult<String> {
    match message.get_item(pyo3::intern!(py, "path"))? {
        Some(item) => item.extract(),
        _ => error_flow!(),
    }
}

#[inline(always)]
fn ws_message_into_rs(py: Python, message: &Bound<PyDict>) -> PyResult<Message> {
    match (
        message.get_item(pyo3::intern!(py, "bytes"))?,
        message.get_item(pyo3::intern!(py, "text"))?,
    ) {
        (Some(item), None) => {
            let data: Cow<[u8]> = item.extract().unwrap_or(EMPTY_BYTES);
            Ok(data[..].into())
        }
        (None, Some(item)) => Ok(Message::Text(item.extract().unwrap_or(EMPTY_STRING))),
        (Some(itemb), Some(itemt)) => match (itemb.extract().unwrap_or(None), itemt.extract().unwrap_or(None)) {
            (Some(msgb), None) => Ok(Message::Binary(msgb)),
            (None, Some(msgt)) => Ok(Message::Text(msgt)),
            _ => error_message!(),
        },
        _ => error_message!(),
    }
}

#[inline(always)]
fn ws_message_into_py(message: Message) -> PyResult<PyObject> {
    match message {
        Message::Binary(message) => Python::with_gil(|py| {
            let dict = PyDict::new_bound(py);
            dict.set_item(pyo3::intern!(py, "type"), pyo3::intern!(py, "websocket.receive"))?;
            dict.set_item(pyo3::intern!(py, "bytes"), PyBytes::new_bound(py, &message[..]))?;
            Ok(dict.to_object(py))
        }),
        Message::Text(message) => Python::with_gil(|py| {
            let dict = PyDict::new_bound(py);
            dict.set_item(pyo3::intern!(py, "type"), pyo3::intern!(py, "websocket.receive"))?;
            dict.set_item(pyo3::intern!(py, "text"), message)?;
            Ok(dict.to_object(py))
        }),
        Message::Close(frame) => Python::with_gil(|py| {
            let close_code: u16 = match frame {
                Some(frame) => frame.code.into(),
                _ => 1005,
            };
            let dict = PyDict::new_bound(py);
            dict.set_item(pyo3::intern!(py, "type"), pyo3::intern!(py, "websocket.disconnect"))?;
            dict.set_item(pyo3::intern!(py, "code"), close_code)?;
            Ok(dict.to_object(py))
        }),
        v => {
            log::warn!("Unsupported websocket message received {:?}", v);
            error_flow!()
        }
    }
}
