use futures::StreamExt;
use http_body_util::BodyExt;
use hyper::body::{self, Bytes};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};
use std::borrow::Cow;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;
use tokio_util::bytes::{BufMut, BytesMut};

use crate::conversion::BytesToPy;
use crate::runtime::RuntimeRef;
use crate::utils::log_application_callable_exception;

const LINE_SPLIT: u8 = u8::from_be_bytes(*b"\n");

enum WSGIBodyBuffering {
    Line,
    Size(usize),
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct WSGIBody {
    rt: RuntimeRef,
    inner: Arc<AsyncMutex<http_body_util::BodyStream<body::Incoming>>>,
    buffer: Arc<Mutex<BytesMut>>,
}

impl WSGIBody {
    pub fn new(rt: RuntimeRef, body: body::Incoming) -> Self {
        Self {
            rt,
            inner: Arc::new(AsyncMutex::new(http_body_util::BodyStream::new(body))),
            buffer: Arc::new(Mutex::new(BytesMut::new())),
        }
    }

    #[allow(clippy::await_holding_lock)]
    async fn fill_buffer(
        stream: Arc<AsyncMutex<http_body_util::BodyStream<body::Incoming>>>,
        buffer: Arc<Mutex<BytesMut>>,
        buffering: WSGIBodyBuffering,
    ) {
        let mut buffer = buffer.lock().unwrap();
        if let WSGIBodyBuffering::Size(size) = buffering {
            if buffer.len() >= size {
                return;
            }
        }

        let mut stream = stream.lock().await;
        loop {
            if let Some(chunk) = stream.next().await {
                let data = chunk
                    .map(|buf| buf.into_data().unwrap_or_default())
                    .unwrap_or(Bytes::new());
                buffer.put(data);
                match buffering {
                    WSGIBodyBuffering::Line => {
                        if !buffer.contains(&LINE_SPLIT) {
                            continue;
                        }
                    }
                    WSGIBodyBuffering::Size(size) => {
                        if buffer.len() < size {
                            continue;
                        }
                    }
                }
            }
            break;
        }
    }

    #[allow(clippy::map_unwrap_or)]
    fn _readline(&self, py: Python) -> Bytes {
        let inner = self.inner.clone();
        py.allow_threads(|| {
            self.rt.inner.block_on(async move {
                WSGIBody::fill_buffer(inner, self.buffer.clone(), WSGIBodyBuffering::Line).await;
            });
        });

        let mut buffer = self.buffer.lock().unwrap();
        buffer
            .iter()
            .position(|&c| c == LINE_SPLIT)
            .map(|next_split| buffer.split_to(next_split).freeze())
            .unwrap_or_else(|| {
                let len = buffer.len();
                buffer.split_to(len).freeze()
            })
    }
}

#[pymethods]
impl WSGIBody {
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(&self, py: Python) -> Option<BytesToPy> {
        let line = self._readline(py);
        match line.len() {
            0 => None,
            _ => Some(BytesToPy(line)),
        }
    }

    #[pyo3(signature = (size=None))]
    fn read(&self, py: Python, size: Option<usize>) -> BytesToPy {
        match size {
            None => {
                let inner = self.inner.clone();
                let data = py.allow_threads(|| {
                    self.rt.inner.block_on(async move {
                        let mut inner = inner.lock().await;
                        BodyExt::collect(&mut *inner)
                            .await
                            .map_or(Bytes::new(), http_body_util::Collected::to_bytes)
                    })
                });
                BytesToPy(data)
            }
            Some(size) => match size {
                0 => BytesToPy(Bytes::new()),
                size => {
                    let inner = self.inner.clone();
                    py.allow_threads(|| {
                        self.rt.inner.block_on(async move {
                            WSGIBody::fill_buffer(inner, self.buffer.clone(), WSGIBodyBuffering::Size(size)).await;
                        });
                    });

                    let mut buffer = self.buffer.lock().unwrap();
                    let limit = buffer.len();
                    let rsize = if size > limit { limit } else { size };
                    let data = buffer.split_to(rsize).freeze();
                    BytesToPy(data)
                }
            },
        }
    }

    #[pyo3(signature = (_size=None))]
    fn readline(&self, py: Python, _size: Option<usize>) -> BytesToPy {
        BytesToPy(self._readline(py))
    }

    #[pyo3(signature = (_hint=None))]
    fn readlines<'p>(&self, py: Python<'p>, _hint: Option<PyObject>) -> Bound<'p, PyList> {
        let inner = self.inner.clone();
        let data = py.allow_threads(|| {
            self.rt.inner.block_on(async move {
                let mut inner = inner.lock().await;
                BodyExt::collect(&mut *inner)
                    .await
                    .map_or(Bytes::new(), http_body_util::Collected::to_bytes)
            })
        });
        let lines: Vec<Bound<PyBytes>> = data
            .split(|&c| c == LINE_SPLIT)
            .map(|item| PyBytes::new_bound(py, item))
            .collect();
        PyList::new_bound(py, lines)
    }
}

pub(crate) struct WSGIResponseBodyIter {
    inner: PyObject,
    closed: bool,
}

impl WSGIResponseBodyIter {
    pub fn new(body: PyObject) -> Self {
        Self {
            inner: body,
            closed: false,
        }
    }

    #[inline]
    fn close_inner(&mut self, py: Python) {
        let _ = self.inner.call_method0(py, pyo3::intern!(py, "close"));
        self.closed = true;
    }
}

impl Drop for WSGIResponseBodyIter {
    fn drop(&mut self) {
        if !self.closed {
            Python::with_gil(|py| self.close_inner(py));
        }
    }
}

impl Iterator for WSGIResponseBodyIter {
    type Item = Result<body::Frame<Bytes>, anyhow::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        Python::with_gil(|py| match self.inner.call_method0(py, pyo3::intern!(py, "__next__")) {
            Ok(chunk_obj) => match chunk_obj.extract::<Cow<[u8]>>(py) {
                Ok(chunk) => {
                    let chunk: Box<[u8]> = chunk.into();
                    Some(Ok(body::Frame::data(Bytes::from(chunk))))
                }
                _ => {
                    self.close_inner(py);
                    None
                }
            },
            Err(err) => {
                if !err.is_instance_of::<pyo3::exceptions::PyStopIteration>(py) {
                    log_application_callable_exception(&err);
                }
                self.close_inner(py);
                None
            }
        })
    }
}
