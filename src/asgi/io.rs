use anyhow::Result;
use futures::{StreamExt, TryStreamExt, sink::SinkExt};
use http_body_util::BodyExt;
use hyper::{
    Response, StatusCode, body,
    header::{HeaderMap, HeaderName, HeaderValue, SERVER as HK_SERVER},
};
use pyo3::{prelude::*, pybacked::PyBackedBytes, types::PyDict};
use std::{
    borrow::Cow,
    sync::{Arc, Mutex, atomic},
};
use tokio::{
    fs::File,
    sync::{Mutex as AsyncMutex, Notify, mpsc, oneshot},
};
use tokio_tungstenite::tungstenite::{Message, protocol::frame as wsframe};
use tokio_util::io::ReaderStream;

use super::{
    errors::{UnsupportedASGIMessage, error_flow, error_message},
    types::ASGIMessageType,
};
use crate::{
    conversion::FutureResultToPy,
    http::{HTTPResponse, HTTPResponseBody, HV_SERVER, response_404},
    runtime::{
        Runtime, RuntimeRef, done_future_into_py, empty_future_into_py, err_future_into_py, future_into_py_futlike,
    },
    ws::{HyperWebsocket, UpgradeData, WSRxStream, WSTxStream},
};

const EMPTY_BYTES: Cow<[u8]> = Cow::Borrowed(b"");
const EMPTY_STRING: String = String::new();
static WS_SUBPROTO_HNAME: &str = "Sec-WebSocket-Protocol";

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct ASGIHTTPProtocol {
    rt: RuntimeRef,
    tx: Mutex<Option<oneshot::Sender<HTTPResponse>>>,
    disconnect_guard: Arc<Notify>,
    request_body: Arc<AsyncMutex<http_body_util::BodyStream<body::Incoming>>>,
    response_started: atomic::AtomicBool,
    response_chunked: atomic::AtomicBool,
    response_intent: Mutex<Option<(u16, HeaderMap)>>,
    body_tx: Mutex<Option<mpsc::UnboundedSender<body::Bytes>>>,
    flow_rx_exhausted: Arc<atomic::AtomicBool>,
    flow_rx_closed: Arc<atomic::AtomicBool>,
    flow_tx_waiter: Arc<Notify>,
    sent_response_code: Arc<atomic::AtomicU16>,
}

impl ASGIHTTPProtocol {
    pub fn new(
        rt: RuntimeRef,
        body: hyper::body::Incoming,
        tx: oneshot::Sender<HTTPResponse>,
        disconnect_guard: Arc<Notify>,
    ) -> Self {
        Self {
            rt,
            tx: Mutex::new(Some(tx)),
            disconnect_guard,
            request_body: Arc::new(AsyncMutex::new(http_body_util::BodyStream::new(body))),
            response_started: false.into(),
            response_chunked: false.into(),
            response_intent: Mutex::new(None),
            body_tx: Mutex::new(None),
            flow_rx_exhausted: Arc::new(atomic::AtomicBool::new(false)),
            flow_rx_closed: Arc::new(atomic::AtomicBool::new(false)),
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
        tx: &mpsc::UnboundedSender<body::Bytes>,
        body: Box<[u8]>,
        close: bool,
    ) -> PyResult<Bound<'p, PyAny>> {
        match tx.send(body.into()) {
            Ok(()) => {
                if close {
                    self.flow_tx_waiter.notify_one();
                }
            }
            Err(err) => {
                if !self.flow_rx_closed.load(atomic::Ordering::Acquire) {
                    log::info!("ASGI transport error: {err:?}");
                }
                self.flow_tx_waiter.notify_one();
            }
        }

        empty_future_into_py(py)
    }

    pub fn tx(&self) -> Option<oneshot::Sender<HTTPResponse>> {
        self.tx.lock().unwrap().take()
    }
}

#[pymethods]
impl ASGIHTTPProtocol {
    fn receive<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        if self.flow_rx_closed.load(atomic::Ordering::Acquire) {
            return done_future_into_py(
                py,
                super::conversion::message_into_py(py, ASGIMessageType::HTTPDisconnect).map(Bound::unbind),
            );
        }

        if self.flow_rx_exhausted.load(atomic::Ordering::Acquire) {
            let guard_tx = self.flow_tx_waiter.clone();
            let guard_disconnect = self.disconnect_guard.clone();
            let disconnected = self.flow_rx_closed.clone();
            return future_into_py_futlike(self.rt.clone(), py, async move {
                tokio::select! {
                    () = guard_tx.notified() => {},
                    () = guard_disconnect.notified() => disconnected.store(true, atomic::Ordering::Release),
                }
                FutureResultToPy::ASGIMessage(ASGIMessageType::HTTPDisconnect)
            });
        }

        let body_ref = self.request_body.clone();
        let guard_tx = self.flow_tx_waiter.clone();
        let guard_disconnect = self.disconnect_guard.clone();
        let exhausted = self.flow_rx_exhausted.clone();
        let disconnected = self.flow_rx_closed.clone();
        future_into_py_futlike(self.rt.clone(), py, async move {
            let mut bodym = body_ref.lock().await;
            let body = &mut *bodym;
            let mut more_body = false;

            let chunk = tokio::select! {
                biased;
                frame = body.next() => match frame {
                    Some(Ok(buf)) => {
                        more_body = true;
                        Some(buf.into_data().unwrap_or_default())
                    }
                    Some(Err(_)) => None,
                    _ => Some(body::Bytes::new()),
                },
                () = guard_disconnect.notified() => {
                    disconnected.store(true, atomic::Ordering::Release);
                    None
                }
            };
            if !more_body {
                exhausted.store(true, atomic::Ordering::Release);
            }

            match chunk {
                Some(data) => FutureResultToPy::ASGIMessage(ASGIMessageType::HTTPRequestBody((data, more_body))),
                _ => {
                    guard_tx.notify_one();
                    FutureResultToPy::ASGIMessage(ASGIMessageType::HTTPDisconnect)
                }
            }
        })
    }

    fn send<'p>(&self, py: Python<'p>, data: &Bound<'p, PyDict>) -> PyResult<Bound<'p, PyAny>> {
        match adapt_message_type(py, data) {
            Ok(ASGIMessageType::HTTPResponseStart(intent)) => {
                if self
                    .response_started
                    .compare_exchange(false, true, atomic::Ordering::Relaxed, atomic::Ordering::Relaxed)
                    .is_err()
                {
                    return error_flow!("Response already started");
                }

                // NOTE: we could definitely avoid this check, and always start a streamed response
                //       and thus get rid of the whole `response_chunked` thing.
                //       But that seems to be ~4% slower when the app actually just want to send 1 msg.
                if !intent
                    .1
                    .get("content-type")
                    .is_some_and(|hv| hv.as_bytes().starts_with(b"text/event-stream"))
                {
                    let mut response_intent = self.response_intent.lock().unwrap();
                    *response_intent = Some(intent);
                    return empty_future_into_py(py);
                }

                self.response_chunked.store(true, atomic::Ordering::Relaxed);
                let (status, headers) = intent;
                let (body_tx, body_rx) = mpsc::unbounded_channel::<body::Bytes>();
                let body_stream = http_body_util::StreamBody::new(
                    tokio_stream::wrappers::UnboundedReceiverStream::new(body_rx)
                        .map(body::Frame::data)
                        .map(Result::Ok),
                );
                *self.body_tx.lock().unwrap() = Some(body_tx.clone());
                self.send_response(status, headers, BodyExt::boxed(body_stream));
                empty_future_into_py(py)
            }
            Ok(ASGIMessageType::HTTPResponseBody((body, more))) => {
                match (
                    self.response_started.load(atomic::Ordering::Relaxed),
                    more,
                    self.response_chunked.load(atomic::Ordering::Relaxed),
                ) {
                    (true, false, false) => match self.response_intent.lock().unwrap().take() {
                        Some((status, headers)) => {
                            self.send_response(
                                status,
                                headers,
                                http_body_util::Full::new(body::Bytes::from(body))
                                    .map_err(std::convert::Into::into)
                                    .boxed(),
                            );
                            self.flow_tx_waiter.notify_one();
                            empty_future_into_py(py)
                        }
                        _ => error_flow!("Response already finished"),
                    },
                    (true, true, false) => match self.response_intent.lock().unwrap().take() {
                        Some((status, headers)) => {
                            self.response_chunked.store(true, atomic::Ordering::Relaxed);
                            let (body_tx, body_rx) = mpsc::unbounded_channel::<body::Bytes>();
                            let body_stream = http_body_util::StreamBody::new(
                                tokio_stream::wrappers::UnboundedReceiverStream::new(body_rx)
                                    .map(body::Frame::data)
                                    .map(Result::Ok),
                            );
                            *self.body_tx.lock().unwrap() = Some(body_tx.clone());
                            self.send_response(status, headers, BodyExt::boxed(body_stream));
                            self.send_body(py, &body_tx, body, false)
                        }
                        _ => error_flow!("Response already finished"),
                    },
                    (true, true, true) => match &*self.body_tx.lock().unwrap() {
                        Some(tx) => self.send_body(py, tx, body, false),
                        _ => error_flow!("Transport not initialized or closed"),
                    },
                    (true, false, true) => match self.body_tx.lock().unwrap().take() {
                        Some(tx) => match body.is_empty() {
                            false => self.send_body(py, &tx, body, true),
                            true => {
                                self.flow_tx_waiter.notify_one();
                                empty_future_into_py(py)
                            }
                        },
                        _ => error_flow!("Transport not initialized or closed"),
                    },
                    _ => error_flow!("Response not started"),
                }
            }
            Ok(ASGIMessageType::HTTPResponseFile(file_path)) => match (
                self.response_started.load(atomic::Ordering::Relaxed),
                self.tx.lock().unwrap().take(),
            ) {
                (true, Some(tx)) => {
                    let (status, headers) = self.response_intent.lock().unwrap().take().unwrap();
                    // FIXME: to store the actual status in case of 404 this should be re-implemented taking
                    //        into account the following async flow (we return empty future to avoid waiting)
                    self.sent_response_code.store(status, atomic::Ordering::Relaxed);
                    self.rt.spawn(async move {
                        let res = match File::open(&file_path).await {
                            Ok(file) => {
                                let stream = ReaderStream::with_capacity(file, 131_072);
                                let stream_body = http_body_util::StreamBody::new(stream.map_ok(body::Frame::data));
                                let mut res =
                                    Response::new(BodyExt::map_err(stream_body, std::convert::Into::into).boxed());
                                *res.status_mut() = StatusCode::from_u16(status).unwrap();
                                *res.headers_mut() = headers;
                                res
                            }
                            Err(_) => {
                                log::info!("Cannot open file {}", &file_path);
                                response_404()
                            }
                        };
                        let _ = tx.send(res);
                    });
                    empty_future_into_py(py)
                }
                _ => error_flow!("Response not started"),
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
    closeframe: Option<wsframe::CloseFrame>,
}

impl WebsocketDetachedTransport {
    pub fn new(
        consumed: bool,
        rx: Option<WSRxStream>,
        tx: Option<WSTxStream>,
        closeframe: Option<wsframe::CloseFrame>,
    ) -> Self {
        Self {
            consumed,
            rx,
            tx,
            closeframe,
        }
    }

    pub async fn close(&mut self) {
        if let Some(mut tx) = self.tx.take() {
            if let Some(frame) = self.closeframe.take() {
                _ = tx.send(Message::Close(Some(frame))).await;
            }
            if let Err(err) = tx.close().await {
                log::info!("Failed to close websocket with error {err:?}");
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
    init_rx: atomic::AtomicBool,
    init_tx: Arc<atomic::AtomicBool>,
    init_event: Arc<Notify>,
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
            init_rx: false.into(),
            init_tx: Arc::new(false.into()),
            init_event: Arc::new(Notify::new()),
            closed: Arc::new(false.into()),
        }
    }

    #[inline(always)]
    fn accept<'p>(&self, py: Python<'p>, subproto: Option<String>) -> PyResult<Bound<'p, PyAny>> {
        let upgrade = self.upgrade.lock().unwrap().take();
        let websocket = self.websocket.lock().unwrap().take();
        let accepted = self.init_tx.clone();
        let accept_notify = self.init_event.clone();
        let rx = self.ws_rx.clone();
        let tx = self.ws_tx.clone();

        future_into_py_futlike(self.rt.clone(), py, async move {
            if let Some(mut upgrade) = upgrade {
                let upgrade_headers = match subproto {
                    Some(v) => vec![(WS_SUBPROTO_HNAME.to_string(), v)],
                    _ => vec![],
                };
                if (upgrade.send(Some(upgrade_headers)).await).is_ok()
                    && let Some(websocket) = websocket
                    && let Ok(stream) = websocket.await
                {
                    let mut wtx = tx.lock().await;
                    let mut wrx = rx.lock().await;
                    let (tx, rx) = stream.split();
                    *wtx = Some(tx);
                    *wrx = Some(rx);
                    drop(wrx);
                    accepted.store(true, atomic::Ordering::Release);
                    accept_notify.notify_one();
                    return FutureResultToPy::None;
                }
            }
            FutureResultToPy::Err(error_flow!("Connection already upgraded"))
        })
    }

    #[inline(always)]
    fn send_message<'p>(&self, py: Python<'p>, data: Message) -> PyResult<Bound<'p, PyAny>> {
        let transport = self.ws_tx.clone();
        let closed = self.closed.clone();

        future_into_py_futlike(self.rt.clone(), py, async move {
            if let Some(ws) = &mut *(transport.lock().await) {
                match ws.send(data).await {
                    Ok(()) => return FutureResultToPy::None,
                    _ => {
                        if closed.load(atomic::Ordering::Acquire) {
                            log::info!("Attempted to write to a closed websocket");
                            return FutureResultToPy::None;
                        }
                    }
                }
            }
            FutureResultToPy::Err(error_flow!("Transport not initialized or closed"))
        })
    }

    #[inline(always)]
    fn close<'p>(&self, py: Python<'p>, frame: Option<wsframe::CloseFrame>) -> PyResult<Bound<'p, PyAny>> {
        let closed = self.closed.clone();
        let ws_rx = self.ws_rx.clone();
        let ws_tx = self.ws_tx.clone();

        future_into_py_futlike(self.rt.clone(), py, async move {
            if let Some(tx) = ws_tx.lock().await.take() {
                closed.store(true, atomic::Ordering::Release);
                WebsocketDetachedTransport::new(true, ws_rx.lock().await.take(), Some(tx), frame)
                    .close()
                    .await;
            }
            FutureResultToPy::None
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
            WebsocketDetachedTransport::new(self.consumed(), ws_rx.take(), ws_tx.take(), None),
        )
    }
}

#[pymethods]
impl ASGIWebsocketProtocol {
    fn receive<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyAny>> {
        // if it's the first `receive` call, return the connect message
        if self
            .init_rx
            .compare_exchange(false, true, atomic::Ordering::Relaxed, atomic::Ordering::Relaxed)
            .is_ok()
        {
            return done_future_into_py(
                py,
                super::conversion::message_into_py(py, ASGIMessageType::WSConnect).map(Bound::unbind),
            );
        }

        let accepted = self.init_tx.clone();
        let accepted_ev = self.init_event.clone();
        let closed = self.closed.clone();
        let transport = self.ws_rx.clone();

        future_into_py_futlike(self.rt.clone(), py, async move {
            if !accepted.load(atomic::Ordering::Acquire) {
                // need to wait for the protocol to send the accept message and init transport
                accepted_ev.notified().await;
            }

            if let Some(ws) = &mut *(transport.lock().await) {
                while let Some(recv) = ws.next().await {
                    match recv {
                        Ok(Message::Ping(_) | Message::Pong(_)) => {}
                        Ok(message @ Message::Close(_)) => {
                            closed.store(true, atomic::Ordering::Release);
                            return FutureResultToPy::ASGIWSMessage(message);
                        }
                        Ok(message) => return FutureResultToPy::ASGIWSMessage(message),
                        _ => {
                            // treat any recv error as a disconnection
                            closed.store(true, atomic::Ordering::Release);
                            return FutureResultToPy::ASGIWSMessage(Message::Close(None));
                        }
                    }
                }
            }
            FutureResultToPy::Err(error_flow!("Transport not initialized or closed"))
        })
    }

    fn send<'p>(&self, py: Python<'p>, data: &Bound<'p, PyDict>) -> PyResult<Bound<'p, PyAny>> {
        match adapt_message_type(py, data) {
            Ok(ASGIMessageType::WSAccept(subproto)) => self.accept(py, subproto),
            Ok(ASGIMessageType::WSClose(frame)) => self.close(py, frame),
            Ok(ASGIMessageType::WSMessage(message)) => self.send_message(py, message),
            _ => err_future_into_py(py, error_message!()),
        }
    }
}

#[inline(never)]
fn adapt_message_type(py: Python, message: &Bound<PyDict>) -> Result<ASGIMessageType, UnsupportedASGIMessage> {
    match message.get_item(pyo3::intern!(py, "type")) {
        Ok(Some(item)) => {
            let message_type: &str = item.extract()?;
            match message_type {
                "http.response.start" => Ok(ASGIMessageType::HTTPResponseStart((
                    adapt_status_code(py, message)?,
                    adapt_headers(py, message).map_err(|_| UnsupportedASGIMessage)?,
                ))),
                "http.response.body" => Ok(ASGIMessageType::HTTPResponseBody(adapt_body(py, message))),
                "http.response.pathsend" => Ok(ASGIMessageType::HTTPResponseFile(adapt_file(py, message)?)),
                "websocket.accept" => {
                    let subproto: Option<String> = match message.get_item(pyo3::intern!(py, "subprotocol")) {
                        Ok(Some(item)) => item.extract::<String>().map(Some).unwrap_or(None),
                        _ => None,
                    };
                    Ok(ASGIMessageType::WSAccept(subproto))
                }
                "websocket.close" => {
                    let code: wsframe::coding::CloseCode = match message.get_item(pyo3::intern!(py, "code")) {
                        Ok(Some(item)) => item
                            .extract::<u16>()
                            .map(std::convert::Into::into)
                            .unwrap_or(wsframe::coding::CloseCode::Normal),
                        _ => wsframe::coding::CloseCode::Normal,
                    };
                    let reason: String = match message.get_item(pyo3::intern!(py, "reason")) {
                        Ok(Some(item)) => item.extract::<String>().unwrap_or(String::new()),
                        _ => String::new(),
                    };
                    Ok(ASGIMessageType::WSClose(Some(wsframe::CloseFrame {
                        code,
                        reason: reason.into(),
                    })))
                }
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
fn adapt_headers(py: Python, message: &Bound<PyDict>) -> Result<HeaderMap> {
    let mut ret = HeaderMap::new();
    for headers_item in message
        .get_item(pyo3::intern!(py, "headers"))?
        .ok_or(UnsupportedASGIMessage)?
        .try_iter()?
        .flatten()
    {
        let htup = headers_item.extract::<Vec<PyBackedBytes>>()?;
        if htup.len() != 2 {
            return error_message!();
        }
        ret.append(HeaderName::from_bytes(&htup[0])?, HeaderValue::from_bytes(&htup[1])?);
    }
    ret.entry(HK_SERVER).or_insert(HV_SERVER);
    Ok(ret)
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
        _ => error_message!(),
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
        (None, Some(item)) => Ok(Message::Text(item.extract::<String>().unwrap_or(EMPTY_STRING).into())),
        (Some(itemb), Some(itemt)) => match (itemb.is_none(), itemt.is_none()) {
            (false, true) => {
                let data: Box<[u8]> = itemb.extract::<Cow<[u8]>>()?.into();
                Ok(Message::Binary(body::Bytes::from(data)))
            }
            (true, false) => Ok(itemt.extract::<String>()?.into()),
            _ => error_message!(),
        },
        _ => error_message!(),
    }
}
