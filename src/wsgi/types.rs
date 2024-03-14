use futures::Stream;
use hyper::body::Bytes;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};
use std::{
    borrow::Cow,
    cell::RefCell,
    convert::Infallible,
    task::{Context, Poll},
};

use crate::conversion::BytesToPy;

const LINE_SPLIT: u8 = u8::from_be_bytes(*b"\n");

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct WSGIBody {
    inner: RefCell<Bytes>,
}

impl WSGIBody {
    pub fn new(body: Bytes) -> Self {
        Self {
            inner: RefCell::new(body),
        }
    }
}

#[pymethods]
impl WSGIBody {
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(&self) -> Option<BytesToPy> {
        let mut inner = self.inner.borrow_mut();
        inner
            .iter()
            .position(|&c| c == LINE_SPLIT)
            .map(|next_split| BytesToPy(inner.split_to(next_split)))
    }

    #[pyo3(signature = (size=None))]
    fn read(&self, size: Option<usize>) -> BytesToPy {
        match size {
            None => {
                let mut inner = self.inner.borrow_mut();
                let len = inner.len();
                BytesToPy(inner.split_to(len))
            }
            Some(size) => match size {
                0 => BytesToPy(Bytes::new()),
                size => {
                    let mut inner = self.inner.borrow_mut();
                    let limit = inner.len();
                    let rsize = if size > limit { limit } else { size };
                    BytesToPy(inner.split_to(rsize))
                }
            },
        }
    }

    fn readline(&self) -> BytesToPy {
        let mut inner = self.inner.borrow_mut();
        match inner.iter().position(|&c| c == LINE_SPLIT) {
            Some(next_split) => {
                let bytes = inner.split_to(next_split);
                *inner = inner.slice(1..);
                BytesToPy(bytes)
            }
            _ => BytesToPy(Bytes::new()),
        }
    }

    #[pyo3(signature = (_hint=None))]
    fn readlines<'p>(&self, py: Python<'p>, _hint: Option<PyObject>) -> &'p PyList {
        let mut inner = self.inner.borrow_mut();
        let lines: Vec<&PyBytes> = inner
            .split(|&c| c == LINE_SPLIT)
            .map(|item| PyBytes::new(py, item))
            .collect();
        inner.clear();
        PyList::new(py, lines)
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
