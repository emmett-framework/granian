use futures::{StreamExt, sink::SinkExt};
use http_body_util::BodyExt;
use hyper::body;
use pyo3::{prelude::*, pybacked::PyBackedStr};
use std::{
    borrow::Cow,
    pin::Pin,
    sync::{Arc, Mutex, RwLock, atomic},
    task::{Context, Poll},
};
use tokio::sync::{Mutex as AsyncMutex, Notify, SetOnce, mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;

use super::{
    conversion, errors,
    types::{PyResponse, PyResponseBody, PyResponseFile, PyResponseFileRange},
};
use crate::{
    py::interop::PyAbortHandle,
    runtime::{Runtime, RuntimeRef},
    ws::{HyperWebsocket, UpgradeData, WSRxStream, WSTxStream},
};

pub(crate) type WebsocketDetachedTransport = (i32, bool, Option<WSTxStream>);

struct ResponseBodyStream {
    inner: mpsc::Receiver<body::Bytes>,
    closed: Arc<SetOnce<()>>,
}

impl Drop for ResponseBodyStream {
    fn drop(&mut self) {
        _ = self.closed.set(());
    }
}

impl body::Body for ResponseBodyStream {
    type Data = body::Bytes;
    type Error = anyhow::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<body::Frame<Self::Data>, Self::Error>>> {
        self.inner
            .poll_recv(cx)
            .map(|item| item.map(|data| Ok(body::Frame::data(data))))
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_closed() && self.inner.is_empty()
    }

    fn size_hint(&self) -> body::SizeHint {
        body::SizeHint::default()
    }
}

impl ResponseBodyStream {
    fn new(notify: Arc<SetOnce<()>>) -> (mpsc::Sender<body::Bytes>, Self) {
        //: chan capacity 2 (the actual number we need for pipelining) * 2 to have some "margin"
        let (body_tx, body_rx) = mpsc::channel::<body::Bytes>(4);
        let slf = Self {
            inner: body_rx,
            closed: notify,
        };
        (body_tx, slf)
    }
}

#[pyclass(frozen, module = "granian._granian", name = "RSGIHTTPProtocol")]
pub(super) struct HTTPProtocol {
    rt: RuntimeRef,
    disconnect_guard: Arc<SetOnce<()>>,
    tx: Mutex<Option<oneshot::Sender<PyResponse>>>,
    body: Mutex<Option<body::Incoming>>,
    disconnected: Arc<atomic::AtomicBool>,
}

impl HTTPProtocol {
    pub fn new(
        rt: RuntimeRef,
        disconnect_guard: Arc<SetOnce<()>>,
        tx: oneshot::Sender<PyResponse>,
        body: body::Incoming,
    ) -> Self {
        Self {
            rt,
            disconnect_guard,
            tx: Mutex::new(Some(tx)),
            body: Mutex::new(Some(body)),
            disconnected: Arc::new(atomic::AtomicBool::new(false)),
        }
    }

    pub fn tx(&self) -> Option<oneshot::Sender<PyResponse>> {
        self.tx.lock().unwrap().take()
    }
}

#[pymethods]
impl HTTPProtocol {
    fn read(&self, cb_ok: Py<PyAny>, cb_err: Py<PyAny>) -> Option<PyAbortHandle> {
        if let Some(rx) = self.body.lock().unwrap().take() {
            let rt = self.rt.clone();
            let task = self.rt.spawn(async move {
                match rx.collect().await {
                    Ok(data) => rt.spawn_blocking_loopback(move |py| {
                        _ = cb_ok.call1(py, (data.to_bytes(),));
                        drop(cb_err);
                    }),
                    _ => rt.spawn_blocking_loopback(move |py| {
                        _ = cb_err.call0(py);
                        drop(cb_ok);
                    }),
                }
            });
            return Some(PyAbortHandle::new(task.abort_handle()));
        }
        None
    }

    fn reader(&self) -> PyResult<HTTPReader> {
        if let Some(rx) = self.body.lock().unwrap().take() {
            let stream = http_body_util::BodyStream::new(rx);
            return Ok(HTTPReader::new(self.rt.clone(), stream));
        }
        errors::error_proto!()
    }

    #[pyo3(signature = (status=200, headers=vec![]))]
    fn write(&self, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>) {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            _ = tx.send(PyResponse::Body(PyResponseBody::empty(status, headers)));
        }
    }

    #[pyo3(signature = (status=200, headers=vec![], body=vec![].into()))]
    fn write_bytes(&self, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, body: Cow<[u8]>) {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            _ = tx.send(PyResponse::Body(PyResponseBody::from_bytes(
                status,
                headers,
                body.into(),
            )));
        }
    }

    #[pyo3(signature = (status=200, headers=vec![], body=String::new()))]
    fn write_str(&self, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, body: String) {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            _ = tx.send(PyResponse::Body(PyResponseBody::from_string(status, headers, body)));
        }
    }

    #[pyo3(signature = (status, headers, file))]
    fn write_file(&self, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, file: String) {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            _ = tx.send(PyResponse::File(PyResponseFile::new(status, headers, file)));
        }
    }

    #[pyo3(signature = (status, headers, file, start, end))]
    fn write_file_range(
        &self,
        status: u16,
        headers: Vec<(PyBackedStr, PyBackedStr)>,
        file: String,
        start: u64,
        end: u64,
    ) -> PyResult<()> {
        if start >= end {
            return Err(pyo3::exceptions::PyValueError::new_err("Invalid range"));
        }
        if let Some(tx) = self.tx.lock().unwrap().take() {
            _ = tx.send(PyResponse::FileRange(PyResponseFileRange::new(
                status, headers, file, start, end,
            )));
        }
        Ok(())
    }

    fn writer(&self, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>) -> PyResult<HTTPWriter> {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            let (body_tx, body_stream) = ResponseBodyStream::new(self.disconnect_guard.clone());
            _ = tx.send(PyResponse::Body(PyResponseBody::new(
                status,
                headers,
                BodyExt::boxed(body_stream),
            )));
            return Ok(HTTPWriter::new(self.rt.clone(), body_tx));
        }
        errors::error_proto!()
    }

    fn watch(&self, cb: Py<PyAny>) -> Option<PyAbortHandle> {
        if self.disconnected.load(atomic::Ordering::Acquire) {
            return None;
        }

        let guard = self.disconnect_guard.clone();
        let state = self.disconnected.clone();
        let rt = self.rt.clone();
        let task = self.rt.spawn(async move {
            guard.wait().await;
            state.store(true, atomic::Ordering::Release);
            rt.spawn_blocking_loopback(move |py| {
                _ = cb.call0(py);
            });
        });
        Some(PyAbortHandle::new(task.abort_handle()))
    }

    fn close(&self) {
        if let Some(tx) = self.tx() {
            let _ = tx.send(PyResponse::Body(PyResponseBody::empty(500, Vec::new())));
        }
    }
}

#[pyclass(frozen, module = "granian._granian", name = "RSGIHTTPReader")]
pub(super) struct HTTPReader {
    rt: RuntimeRef,
    stream: Arc<AsyncMutex<Option<http_body_util::BodyStream<body::Incoming>>>>,
}

impl HTTPReader {
    fn new(rt: RuntimeRef, stream: http_body_util::BodyStream<body::Incoming>) -> Self {
        Self {
            rt,
            stream: Arc::new(AsyncMutex::new(Some(stream))),
        }
    }
}

#[pymethods]
impl HTTPReader {
    fn read(&self, cb: Py<PyAny>) -> PyAbortHandle {
        let rth = self.rt.clone();
        let stream = self.stream.clone();
        let task = self.rt.spawn(async move {
            let guard = &mut stream.lock().await;
            if let Some(stream) = guard.as_mut() {
                match stream.next().await {
                    Some(chunk) => {
                        let chunk = chunk.map_or(body::Bytes::new(), |buf| buf.into_data().unwrap_or_default());
                        let eof = chunk.is_empty();
                        rth.spawn_blocking_loopback(move |py| {
                            _ = cb.call1(py, (chunk, eof));
                        });
                    }
                    _ => {
                        _ = guard.take();
                        rth.spawn_blocking_loopback(move |py| {
                            _ = cb.call1(py, (body::Bytes::new(), true));
                        });
                    }
                }
            }
        });
        PyAbortHandle::new(task.abort_handle())
    }
}

#[pyclass(frozen, module = "granian._granian", name = "RSGIHTTPWriter")]
pub(super) struct HTTPWriter {
    rt: RuntimeRef,
    stream: mpsc::Sender<body::Bytes>,
}

impl HTTPWriter {
    fn new(rt: RuntimeRef, stream: mpsc::Sender<body::Bytes>) -> Self {
        Self { rt, stream }
    }

    #[inline(always)]
    fn write<T>(&self, frame: T, cb_ok: Py<PyAny>, cb_err: Py<PyAny>) -> PyAbortHandle
    where
        bytes::Bytes: From<T>,
        T: Send + 'static,
    {
        let rt = self.rt.clone();
        let stream = self.stream.clone();
        let task = self.rt.spawn(async move {
            match stream.send(frame.into()).await {
                Ok(()) => rt.spawn_blocking_loopback(move |py| {
                    _ = cb_ok.call0(py);
                    drop(cb_err);
                }),
                _ => rt.spawn_blocking_loopback(move |py| {
                    _ = cb_err.call0(py);
                    drop(cb_ok);
                }),
            }
        });
        PyAbortHandle::new(task.abort_handle())
    }
}

#[pymethods]
impl HTTPWriter {
    fn write_bytes(&self, data: Cow<[u8]>, cb_ok: Py<PyAny>, cb_err: Py<PyAny>) -> PyAbortHandle {
        let bdata: Box<[u8]> = data.into();
        self.write(bdata, cb_ok, cb_err)
    }

    fn write_str(&self, data: String, cb_ok: Py<PyAny>, cb_err: Py<PyAny>) -> PyAbortHandle {
        self.write(data, cb_ok, cb_err)
    }
}

#[pyclass(frozen, module = "granian._granian", name = "RSGIWebsocketProtocol")]
pub(crate) struct WebsocketProtocol {
    rt: RuntimeRef,
    tx: Mutex<Option<oneshot::Sender<WebsocketDetachedTransport>>>,
    disconnect_guard: Arc<Notify>,
    websocket: Arc<AsyncMutex<HyperWebsocket>>,
    upgrade: RwLock<Option<UpgradeData>>,
    // closed: Arc<atomic::AtomicBool>,
    transport: Arc<AsyncMutex<Option<WSTxStream>>>,
}

impl WebsocketProtocol {
    pub fn new(
        rt: RuntimeRef,
        tx: oneshot::Sender<WebsocketDetachedTransport>,
        websocket: HyperWebsocket,
        upgrade: UpgradeData,
        disconnect_guard: Arc<Notify>,
    ) -> Self {
        Self {
            rt,
            tx: Mutex::new(Some(tx)),
            disconnect_guard,
            websocket: Arc::new(AsyncMutex::new(websocket)),
            upgrade: RwLock::new(Some(upgrade)),
            // closed: Arc::new(false.into()),
            transport: Arc::new(AsyncMutex::new(None)),
        }
    }

    fn consumed(&self) -> bool {
        self.upgrade.read().unwrap().is_none()
    }
}

#[pymethods]
impl WebsocketProtocol {
    #[pyo3(signature = (status=None))]
    pub fn close(&self, status: Option<i32>) {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            // self.closed.store(true, atomic::Ordering::Release);
            let transport = self.transport.clone();
            let consumed = self.consumed();

            self.rt.spawn(async move {
                let mut handle = None;
                let mut transport = transport.lock().await;
                if let Some(transport) = transport.take() {
                    handle = Some(transport);
                }

                let _ = tx.send((status.unwrap_or(0), consumed, handle));
            });
        }
    }

    fn accept(&self, cb_ok: Py<PyAny>, cb_err: Py<PyAny>) -> PyAbortHandle {
        let rth = self.rt.clone();
        let dg = self.disconnect_guard.clone();
        let mut upgrade = self.upgrade.write().unwrap().take().unwrap();
        let transport = self.websocket.clone();
        let itransport = self.transport.clone();

        let task = self.rt.spawn(async move {
            let mut ws = transport.lock().await;
            if let Ok(()) = upgrade.send(None, None, None).await
                && let Ok(stream) = (&mut *ws).await
            {
                let (stx, srx) = stream.split();
                {
                    let mut guard = itransport.lock().await;
                    *guard = Some(stx);
                }
                let rthr = rth.clone();
                let rthw = rth.clone();
                rth.spawn_blocking_loopback(move |py| {
                    _ = cb_ok.call1(
                        py,
                        (
                            WebsocketReader::new(rthr, dg, srx),
                            WebsocketWriter::new(rthw, itransport),
                        ),
                    );
                    drop(cb_err);
                });
                return;
            }
            rth.spawn_blocking_loopback(move |py| {
                _ = cb_err.call0(py);
                drop(cb_ok);
            });
        });
        PyAbortHandle::new(task.abort_handle())
    }
}

#[pyclass(frozen, module = "granian._granian", name = "RSGIWebsocketReader")]
pub(crate) struct WebsocketReader {
    rt: RuntimeRef,
    dg: Arc<Notify>,
    stream: Arc<AsyncMutex<WSRxStream>>,
}

impl WebsocketReader {
    pub fn new(rt: RuntimeRef, dg: Arc<Notify>, stream: WSRxStream) -> Self {
        Self {
            rt,
            dg,
            stream: Arc::new(AsyncMutex::new(stream)),
        }
    }
}

#[pymethods]
impl WebsocketReader {
    fn read(&self, cb_ok: Py<PyAny>, cb_err: Py<PyAny>) -> PyAbortHandle {
        let rth = self.rt.clone();
        let transport = self.stream.clone();
        let dg = self.dg.clone();
        let task = self.rt.spawn(async move {
            if let Ok(mut stream) = transport.try_lock() {
                while let Some(recv) = tokio::select! {
                    biased;
                    recv = stream.next() => recv,
                    () = dg.notified() => Some(Err(tokio_tungstenite::tungstenite::Error::ConnectionClosed))
                } {
                    match recv {
                        Ok(Message::Ping(_) | Message::Pong(_)) => {}
                        Ok(message) => {
                            rth.spawn_blocking_loopback(move |py| {
                                _ = cb_ok.call1(py, (conversion::ws_message_into_py(py, message).unwrap(),));
                                drop(cb_err);
                            });
                            return;
                        }
                        _ => break,
                    }
                }
            }
            rth.spawn_blocking_loopback(move |py| {
                _ = cb_err.call0(py);
                drop(cb_ok);
            });
        });
        PyAbortHandle::new(task.abort_handle())
    }
}

#[pyclass(frozen, module = "granian._granian", name = "RSGIWebsocketWriter")]
pub(crate) struct WebsocketWriter {
    rt: RuntimeRef,
    stream: Arc<AsyncMutex<Option<WSTxStream>>>,
}

impl WebsocketWriter {
    pub fn new(rt: RuntimeRef, stream: Arc<AsyncMutex<Option<WSTxStream>>>) -> Self {
        Self { rt, stream }
    }
}

#[pymethods]
impl WebsocketWriter {
    fn write_bytes(&self, data: Cow<[u8]>, cb_ok: Py<PyAny>, cb_err: Py<PyAny>) -> PyAbortHandle {
        let rth = self.rt.clone();
        let transport = self.stream.clone();
        let bdata: Box<[u8]> = data.into();
        let task = self.rt.spawn(async move {
            if let Some(stream) = &mut *(transport.lock().await)
                && let Ok(()) = stream.send(bdata[..].into()).await
            {
                rth.spawn_blocking_loopback(move |py| {
                    _ = cb_ok.call0(py);
                    drop(cb_err);
                });
                return;
            }
            rth.spawn_blocking_loopback(move |py| {
                _ = cb_err.call0(py);
                drop(cb_ok);
            });
        });
        PyAbortHandle::new(task.abort_handle())
    }

    fn write_str(&self, data: String, cb_ok: Py<PyAny>, cb_err: Py<PyAny>) -> PyAbortHandle {
        let rth = self.rt.clone();
        let transport = self.stream.clone();
        let task = self.rt.spawn(async move {
            if let Some(stream) = &mut *(transport.lock().await)
                && let Ok(()) = stream.send(data.into()).await
            {
                rth.spawn_blocking_loopback(move |py| {
                    _ = cb_ok.call0(py);
                    drop(cb_err);
                });
                return;
            }
            rth.spawn_blocking_loopback(move |py| {
                _ = cb_err.call0(py);
                drop(cb_ok);
            });
        });
        PyAbortHandle::new(task.abort_handle())
    }
}
