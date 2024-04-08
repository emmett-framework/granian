use futures::{Stream, StreamExt};
use http_body_util::BodyExt;
use hyper::body::{self, Bytes};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};
use std::sync::{Arc, Mutex};
use std::{
    borrow::Cow,
    convert::Infallible,
    task::{Context, Poll},
};
use tokio::sync::Mutex as AsyncMutex;
use tokio_util::bytes::BytesMut;

use crate::conversion::BytesToPy;
use crate::runtime::RuntimeRef;

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

    async fn fill_buffer(
        stream: Arc<AsyncMutex<http_body_util::BodyStream<body::Incoming>>>,
        buffer: Arc<Mutex<BytesMut>>,
        buffering: WSGIBodyBuffering,
    ) {
        let mut stream = stream.lock().await;
        loop {
            if let Some(chunk) = stream.next().await {
                let data = chunk
                    .map(|buf| buf.into_data().unwrap_or_default())
                    .unwrap_or(Bytes::new());
                let mut buffer = buffer.lock().unwrap();
                buffer.extend_from_slice(data.as_ref());
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
        let buffer = self.buffer.clone();
        py.allow_threads(|| {
            self.rt.inner.block_on(async move {
                WSGIBody::fill_buffer(inner, buffer, WSGIBodyBuffering::Line).await;
            });
        });

        let mut buffer = self.buffer.lock().unwrap();
        buffer
            .iter()
            .position(|&c| c == LINE_SPLIT)
            .map(|next_split| buffer.split_to(next_split).into())
            .unwrap_or_else(|| {
                let len = buffer.len();
                buffer.split_to(len).into()
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
                    let buffer = self.buffer.clone();
                    py.allow_threads(|| {
                        self.rt.inner.block_on(async move {
                            WSGIBody::fill_buffer(inner, buffer, WSGIBodyBuffering::Size(size)).await;
                        });
                    });
                    let mut buffer = self.buffer.lock().unwrap();
                    let limit = buffer.len();
                    let rsize = if size > limit { limit } else { size };
                    BytesToPy(buffer.split_to(rsize).into())
                }
            },
        }
    }

    fn readline(&self, py: Python) -> BytesToPy {
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
}

impl WSGIResponseBodyIter {
    pub fn new(body: PyObject) -> Self {
        Self { inner: body }
    }

    #[inline]
    fn close_inner(&self, py: Python) {
        let _ = self.inner.call_method0(py, pyo3::intern!(py, "close"));
    }
}

impl Stream for WSGIResponseBodyIter {
    type Item = Result<Box<[u8]>, Infallible>;

    fn poll_next(self: std::pin::Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let ret = Python::with_gil(|py| match self.inner.call_method0(py, pyo3::intern!(py, "__next__")) {
            Ok(chunk_obj) => match chunk_obj.extract::<Cow<[u8]>>(py) {
                Ok(chunk) => Some(Ok(chunk.into())),
                _ => {
                    self.close_inner(py);
                    None
                }
            },
            Err(err) => {
                if err.is_instance_of::<pyo3::exceptions::PyStopIteration>(py) {
                    self.close_inner(py);
                }
                None
            }
        });
        Poll::Ready(ret)
    }
}
