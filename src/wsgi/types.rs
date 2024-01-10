use futures::Stream;
use hyper::{
    body::Bytes,
    header::{HeaderMap, CONTENT_LENGTH, CONTENT_TYPE},
    Body, Method, Request, Uri, Version,
};
use percent_encoding::percent_decode_str;
use pyo3::types::{PyBytes, PyDict, PyList};
use pyo3::{prelude::*, types::IntoPyDict};
use std::{
    borrow::Cow,
    net::{IpAddr, SocketAddr},
    task::{Context, Poll},
};

const LINE_SPLIT: u8 = u8::from_be_bytes(*b"\n");

#[pyclass(module = "granian._granian")]
pub(crate) struct WSGIBody {
    inner: Bytes,
}

impl WSGIBody {
    pub fn new(body: Bytes) -> Self {
        Self { inner: body }
    }
}

#[pymethods]
impl WSGIBody {
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__<'p>(&mut self, py: Python<'p>) -> Option<&'p PyBytes> {
        match self.inner.iter().position(|&c| c == LINE_SPLIT) {
            Some(next_split) => {
                let bytes = self.inner.split_to(next_split);
                Some(PyBytes::new(py, &bytes))
            }
            _ => None,
        }
    }

    #[pyo3(signature = (size=None))]
    fn read<'p>(&mut self, py: Python<'p>, size: Option<usize>) -> &'p PyBytes {
        match size {
            None => {
                let bytes = self.inner.split_to(self.inner.len());
                PyBytes::new(py, &bytes[..])
            }
            Some(size) => match size {
                0 => PyBytes::new(py, b""),
                size => {
                    let limit = self.inner.len();
                    let rsize = if size > limit { limit } else { size };
                    let bytes = self.inner.split_to(rsize);
                    PyBytes::new(py, &bytes[..])
                }
            },
        }
    }

    fn readline<'p>(&mut self, py: Python<'p>) -> &'p PyBytes {
        match self.inner.iter().position(|&c| c == LINE_SPLIT) {
            Some(next_split) => {
                let bytes = self.inner.split_to(next_split);
                self.inner = self.inner.slice(1..);
                PyBytes::new(py, &bytes[..])
            }
            _ => PyBytes::new(py, b""),
        }
    }

    #[pyo3(signature = (_hint=None))]
    fn readlines<'p>(&mut self, py: Python<'p>, _hint: Option<PyObject>) -> &'p PyList {
        let lines: Vec<&PyBytes> = self
            .inner
            .split(|&c| c == LINE_SPLIT)
            .map(|item| PyBytes::new(py, item))
            .collect();
        self.inner.clear();
        PyList::new(py, lines)
    }
}

#[pyclass(module = "granian._granian")]
pub(crate) struct WSGIScope {
    http_version: Version,
    scheme: String,
    method: String,
    uri: Uri,
    server_ip: IpAddr,
    server_port: u16,
    client: String,
    headers: HeaderMap,
    body: Option<Bytes>,
}

impl WSGIScope {
    pub async fn new(scheme: &str, server: SocketAddr, client: SocketAddr, request: Request<Body>) -> Self {
        let http_version = request.version();
        let method = request.method().clone();
        let uri = request.uri().clone();
        let headers = request.headers().clone();

        let body = match method {
            Method::HEAD | Method::GET | Method::OPTIONS => Bytes::new(),
            _ => hyper::body::to_bytes(request).await.unwrap_or(Bytes::new()),
        };

        Self {
            http_version,
            scheme: scheme.to_string(),
            method: method.to_string(),
            uri,
            server_ip: server.ip(),
            server_port: server.port(),
            client: client.to_string(),
            headers,
            body: Some(body),
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
    fn to_environ<'p>(&mut self, py: Python<'p>, ret: &'p PyDict) -> PyResult<&'p PyDict> {
        let (
            path,
            query_string,
            http_version,
            server,
            client,
            scheme,
            method,
            content_type,
            content_len,
            headers,
            body,
        ) = py.allow_threads(|| {
            let (path, query_string) = self
                .uri
                .path_and_query()
                .map_or_else(|| ("", ""), |pq| (pq.path(), pq.query().unwrap_or("")));
            let content_type = self.headers.remove(CONTENT_TYPE);
            let content_len = self.headers.remove(CONTENT_LENGTH);
            let mut headers = Vec::with_capacity(self.headers.len());

            for (key, val) in &self.headers {
                headers.push((
                    format!("HTTP_{}", key.as_str().replace('-', "_").to_uppercase()),
                    val.to_str().unwrap_or_default(),
                ));
            }

            (
                percent_decode_str(path).decode_utf8().unwrap(),
                query_string,
                self.py_http_version(),
                (self.server_ip.to_string(), self.server_port.to_string()),
                &self.client[..],
                &self.scheme[..],
                &self.method[..],
                content_type,
                content_len,
                headers,
                WSGIBody::new(self.body.take().unwrap()),
            )
        });

        ret.set_item(pyo3::intern!(py, "SERVER_PROTOCOL"), http_version)?;
        ret.set_item(pyo3::intern!(py, "SERVER_NAME"), server.0)?;
        ret.set_item(pyo3::intern!(py, "SERVER_PORT"), server.1)?;
        ret.set_item(pyo3::intern!(py, "REMOTE_ADDR"), client)?;
        ret.set_item(pyo3::intern!(py, "REQUEST_METHOD"), method)?;
        ret.set_item(pyo3::intern!(py, "PATH_INFO"), path)?;
        ret.set_item(pyo3::intern!(py, "QUERY_STRING"), query_string)?;
        ret.set_item(pyo3::intern!(py, "wsgi.url_scheme"), scheme)?;
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

    fn close_inner(&self, py: Python) {
        let _ = self.inner.call_method0(py, pyo3::intern!(py, "close"));
    }
}

impl Stream for WSGIResponseBodyIter {
    type Item = PyResult<Box<[u8]>>;

    fn poll_next(self: std::pin::Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Python::with_gil(|py| match self.inner.call_method0(py, pyo3::intern!(py, "__next__")) {
            Ok(chunk_obj) => match chunk_obj.extract::<Cow<[u8]>>(py) {
                Ok(chunk) => Poll::Ready(Some(Ok(chunk.into()))),
                _ => {
                    self.close_inner(py);
                    Poll::Ready(None)
                }
            },
            Err(err) => {
                if err.is_instance_of::<pyo3::exceptions::PyStopIteration>(py) {
                    self.close_inner(py);
                }
                Poll::Ready(None)
            }
        })
    }
}
