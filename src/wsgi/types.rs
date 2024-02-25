use futures::Stream;
use http_body_util::BodyExt;
use hyper::{
    body::Bytes,
    header::{HeaderMap, CONTENT_LENGTH, CONTENT_TYPE, HOST},
    http::uri::Authority,
    Method, Uri, Version,
};
use percent_encoding::percent_decode_str;
use pyo3::types::{PyBytes, PyDict, PyList};
use pyo3::{prelude::*, types::IntoPyDict};
use std::{
    borrow::Cow,
    cell::RefCell,
    convert::Infallible,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use crate::{conversion::BytesToPy, http::HTTPRequest};

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

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct WSGIScope {
    http_version: Version,
    scheme: Arc<str>,
    method: Arc<str>,
    uri: Uri,
    server_ip: IpAddr,
    server_port: u16,
    client: String,
    headers: Mutex<HeaderMap>,
    body: Mutex<Option<Bytes>>,
}

impl WSGIScope {
    pub async fn new(scheme: &str, server: SocketAddr, client: SocketAddr, request: HTTPRequest) -> Self {
        let http_version = request.version();
        let method = request.method().clone();
        let uri = request.uri().clone();
        let headers = Mutex::new(request.headers().clone());

        let body = match method {
            Method::HEAD | Method::GET | Method::OPTIONS => Bytes::new(),
            _ => request
                .collect()
                .await
                .map_or(Bytes::new(), http_body_util::Collected::to_bytes),
        };

        Self {
            http_version,
            scheme: scheme.into(),
            method: method.as_str().into(),
            uri,
            server_ip: server.ip(),
            server_port: server.port(),
            client: client.to_string(),
            headers,
            body: Mutex::new(Some(body)),
        }
    }

    #[inline(always)]
    fn py_http_version(&self) -> String {
        match self.http_version {
            Version::HTTP_10 => "HTTP/1",
            Version::HTTP_11 => "HTTP/1.1",
            Version::HTTP_2 => "HTTP/2",
            Version::HTTP_3 => "HTTP/3",
            _ => "HTTP/1",
        }
        .into()
    }
}

#[pymethods]
impl WSGIScope {
    fn to_environ<'p>(&self, py: Python<'p>, ret: &'p PyDict) -> PyResult<&'p PyDict> {
        let (path, query_string, http_version, server, client, content_type, content_len, headers, body) = py
            .allow_threads(|| {
                let (path, query_string) = self
                    .uri
                    .path_and_query()
                    .map_or_else(|| ("", ""), |pq| (pq.path(), pq.query().unwrap_or("")));
                let mut source_headers = self.headers.lock().unwrap();
                let content_type = source_headers.remove(CONTENT_TYPE);
                let content_len = source_headers.remove(CONTENT_LENGTH);
                let mut headers = Vec::with_capacity(source_headers.len());

                for (key, val) in source_headers.iter() {
                    headers.push((
                        format!("HTTP_{}", key.as_str().replace('-', "_").to_uppercase()),
                        val.to_str().unwrap_or_default().to_owned(),
                    ));
                }
                if !source_headers.contains_key(HOST) {
                    let host = self.uri.authority().map_or("", Authority::as_str);
                    headers.push(("HTTP_HOST".to_string(), host.to_owned()));
                }

                (
                    percent_decode_str(path).decode_utf8().unwrap(),
                    query_string,
                    self.py_http_version(),
                    (self.server_ip.to_string(), self.server_port.to_string()),
                    &self.client[..],
                    content_type,
                    content_len,
                    headers,
                    WSGIBody::new(self.body.lock().unwrap().take().unwrap()),
                )
            });

        ret.set_item(pyo3::intern!(py, "SERVER_PROTOCOL"), http_version)?;
        ret.set_item(pyo3::intern!(py, "SERVER_NAME"), server.0)?;
        ret.set_item(pyo3::intern!(py, "SERVER_PORT"), server.1)?;
        ret.set_item(pyo3::intern!(py, "REMOTE_ADDR"), client)?;
        ret.set_item(pyo3::intern!(py, "REQUEST_METHOD"), &*self.method)?;
        ret.set_item(pyo3::intern!(py, "PATH_INFO"), path)?;
        ret.set_item(pyo3::intern!(py, "QUERY_STRING"), query_string)?;
        ret.set_item(pyo3::intern!(py, "wsgi.url_scheme"), &*self.scheme)?;
        ret.set_item(pyo3::intern!(py, "wsgi.input"), Py::new(py, body)?)?;

        if let Some(content_type) = content_type {
            ret.set_item(
                pyo3::intern!(py, "CONTENT_TYPE"),
                content_type.to_str().unwrap_or_default(),
            )?;
        }
        if let Some(content_len) = content_len {
            ret.set_item(
                pyo3::intern!(py, "CONTENT_LENGTH"),
                content_len.to_str().unwrap_or_default(),
            )?;
        }

        ret.update(headers.into_py_dict(py).as_mapping())?;

        Ok(ret)
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
