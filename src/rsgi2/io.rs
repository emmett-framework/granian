use futures::StreamExt;
use http_body_util::BodyExt;
use hyper::body;
use pyo3::{prelude::*, pybacked::PyBackedStr};
use std::{
    borrow::Cow,
    sync::{Arc, Mutex, atomic},
};
use tokio::sync::{Mutex as AsyncMutex, Notify, mpsc, oneshot};

use crate::{
    rsgi::types::{PyResponse, PyResponseBody, PyResponseFile, PyResponseFileRange},
    runtime::Runtime,
};

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct RSGI2HTTPProtocol {
    rt: crate::runtime::RuntimeRef,
    tx: Mutex<Option<oneshot::Sender<PyResponse>>>,
    disconnect_guard: Arc<Notify>,
    body: Mutex<Option<body::Incoming>>,
    disconnected: Arc<atomic::AtomicBool>,
}

impl RSGI2HTTPProtocol {
    pub fn new(
        rt: crate::runtime::RuntimeRef,
        tx: oneshot::Sender<PyResponse>,
        body: body::Incoming,
        disconnect_guard: Arc<Notify>,
    ) -> Self {
        Self {
            rt,
            tx: Mutex::new(Some(tx)),
            disconnect_guard,
            body: Mutex::new(Some(body)),
            disconnected: Arc::new(atomic::AtomicBool::new(false)),
        }
    }

    pub fn tx(&self) -> Option<oneshot::Sender<PyResponse>> {
        self.tx.lock().unwrap().take()
    }
}

#[pymethods]
impl RSGI2HTTPProtocol {
    fn read(&self, cb: Py<PyAny>) -> Option<super::callbacks::PyAbortHandle> {
        if let Some(rx) = self.body.lock().unwrap().take() {
            // let cb = self.cb.clone();
            let rt = self.rt.clone();
            let task = self.rt.spawn(async move {
                match rx.collect().await {
                    Ok(data) => rt.spawn_blocking(move |py| {
                        _ = cb.call1(py, (data.to_bytes(), true));
                        // println!("read cb res {ret:?}");
                    }),
                    _ => rt.spawn_blocking(move |py| {
                        // we need on_error!
                    }),
                }
            });
            return Some(super::callbacks::PyAbortHandle::new(task.abort_handle()));
        }
        None
    }

    fn reader(&self) -> PyResult<RSGIHTTPReader> {
        if let Some(rx) = self.body.lock().unwrap().take() {
            let stream = http_body_util::BodyStream::new(rx);
            return Ok(RSGIHTTPReader::new(self.rt.clone(), stream));
        }
        crate::rsgi::errors::error_proto!()
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

    fn writer(&self, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>) -> PyResult<RSGIHTTPWriter> {
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
            return Ok(RSGIHTTPWriter::new(body_tx));
        }
        crate::rsgi::errors::error_proto!()
    }

    fn watch(&self, cb: Py<PyAny>) -> Option<super::callbacks::PyAbortHandle> {
        if self.disconnected.load(atomic::Ordering::Acquire) {
            return None;
        }

        let disconnect = self.disconnect_guard.clone();
        let state = self.disconnected.clone();
        let rt = self.rt.clone();

        let task = self.rt.spawn(async move {
            disconnect.notified().await;
            state.store(true, atomic::Ordering::Release);
            rt.spawn_blocking(move |py| {
                _ = cb.call0(py);
            });
        });
        Some(super::callbacks::PyAbortHandle::new(task.abort_handle()))
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(super) struct RSGIHTTPReader {
    rt: crate::runtime::RuntimeRef,
    stream: Arc<AsyncMutex<Option<http_body_util::BodyStream<body::Incoming>>>>,
    // cb: Arc<Py<PyAny>>,
}

impl RSGIHTTPReader {
    fn new(rt: crate::runtime::RuntimeRef, stream: http_body_util::BodyStream<body::Incoming>) -> Self {
        Self {
            rt,
            stream: Arc::new(AsyncMutex::new(Some(stream))),
            // cb,
        }
    }
}

#[pymethods]
impl RSGIHTTPReader {
    fn read(&self, cb: Py<PyAny>) -> super::callbacks::PyAbortHandle {
        let rth = self.rt.clone();
        let stream = self.stream.clone();
        // let cb = self.cb.clone();

        let task = self.rt.spawn(async move {
            let guard = &mut stream.lock().await;
            if let Some(stream) = guard.as_mut() {
                match stream.next().await {
                    Some(chunk) => {
                        let chunk = chunk
                            .map(|buf| buf.into_data().unwrap_or_default())
                            .unwrap_or(body::Bytes::new());
                        // let eof = chunk.is_empty();
                        rth.spawn_blocking(move |py| {
                            // should we compute eof here?
                            _ = cb.call1(py, (chunk, false));
                        });
                    }
                    _ => {
                        _ = guard.take();
                        rth.spawn_blocking(move |py| {
                            _ = cb.call1(py, (body::Bytes::new(), true));
                        });
                    }
                }
            };
        });
        super::callbacks::PyAbortHandle::new(task.abort_handle())
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(super) struct RSGIHTTPWriter {
    stream: mpsc::UnboundedSender<body::Bytes>,
}

impl RSGIHTTPWriter {
    fn new(stream: mpsc::UnboundedSender<body::Bytes>) -> Self {
        Self { stream }
    }
}

#[pymethods]
impl RSGIHTTPWriter {
    fn write_bytes(&self, data: Cow<[u8]>) -> PyResult<()> {
        let bdata = body::Bytes::from(std::convert::Into::<Box<[u8]>>::into(data));
        if self.stream.send(bdata).is_err() {
            return crate::rsgi::errors::error_stream!();
        }
        Ok(())
    }

    fn write_str(&self, data: String) -> PyResult<()> {
        if self.stream.send(body::Bytes::from(data)).is_err() {
            return crate::rsgi::errors::error_stream!();
        }
        Ok(())
    }
}
