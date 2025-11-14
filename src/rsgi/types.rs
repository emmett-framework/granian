use anyhow::Result;
use futures::TryStreamExt;
use http_body_util::BodyExt;
use hyper::{
    Method, Uri, Version,
    body::Bytes,
    header::{HeaderMap, HeaderName, HeaderValue, SERVER as HK_SERVER},
    http::uri::Authority,
};
use percent_encoding::percent_decode_str;
use pyo3::types::{PyBytes, PyIterator, PyList, PyString};
use pyo3::{prelude::*, pybacked::PyBackedStr};
use std::borrow::Cow;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{
    http::{HTTPProto, HTTPResponseBody, HV_SERVER, empty_body, response_404, response_500},
    net::SockAddr,
};

const RSGI_PROTO_VERSION: &str = "1.6";

#[pyclass(frozen, module = "granian._granian")]
#[derive(Clone)]
pub(crate) struct RSGIHeaders {
    inner: HeaderMap,
}

impl RSGIHeaders {
    pub fn new(map: HeaderMap) -> Self {
        Self { inner: map }
    }
}

#[pymethods]
impl RSGIHeaders {
    fn keys(&self) -> Vec<&str> {
        let mut ret = Vec::with_capacity(self.inner.keys_len());
        for key in self.inner.keys() {
            ret.push(key.as_str());
        }
        ret
    }

    fn values(&self) -> Vec<&str> {
        let mut ret = Vec::with_capacity(self.inner.len());
        for val in self.inner.values() {
            if let Ok(v) = val.to_str() {
                ret.push(v);
            }
        }
        ret
    }

    fn items(&self) -> Vec<(&str, &str)> {
        let mut ret = Vec::with_capacity(self.inner.len());
        for (key, val) in &self.inner {
            if let Ok(v) = val.to_str() {
                ret.push((key.as_str(), v));
            }
        }
        ret
    }

    fn __contains__(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    fn __getitem__(&self, key: &str) -> Result<&str> {
        if let Some(value) = self.inner.get(key) {
            return Ok(value.to_str().unwrap_or(""));
        }
        Err(pyo3::exceptions::PyKeyError::new_err(key.to_owned()).into())
    }

    fn __iter__<'p>(&self, py: Python<'p>) -> PyResult<Bound<'p, PyIterator>> {
        PyIterator::from_object(PyList::new(py, self.keys())?.as_any())
    }

    fn __len__(&self) -> usize {
        self.inner.keys_len()
    }

    #[pyo3(signature = (key, default=None))]
    fn get(&self, py: Python, key: &str, default: Option<Py<PyAny>>) -> Option<Py<PyAny>> {
        if let Some(val) = self.inner.get(key)
            && let Ok(v) = val.to_str()
        {
            return Some(PyString::new(py, v).into());
        }
        default
    }

    #[pyo3(signature = (key))]
    fn get_all<'p>(&self, py: Python<'p>, key: &'p str) -> PyResult<Bound<'p, PyList>> {
        PyList::new(
            py,
            self.inner
                .get_all(key)
                .iter()
                .flat_map(|v| v.to_str())
                .map(|v| PyString::new(py, v))
                .collect::<Vec<Bound<PyString>>>(),
        )
    }
}

macro_rules! rsgi_scope_cls {
    ($name:ident, $proto:expr) => {
        #[pyclass(frozen, module = "granian._granian")]
        pub(crate) struct $name {
            http_version: Version,
            scheme: HTTPProto,
            method: Method,
            uri: Uri,
            server: SockAddr,
            client: SockAddr,
            #[pyo3(get)]
            headers: RSGIHeaders,
        }

        impl $name {
            pub fn new(
                http_version: Version,
                scheme: HTTPProto,
                uri: Uri,
                method: Method,
                server: SockAddr,
                client: SockAddr,
                headers: HeaderMap,
            ) -> Self {
                Self {
                    http_version,
                    scheme,
                    method,
                    uri,
                    server,
                    client,
                    headers: RSGIHeaders::new(headers),
                }
            }
        }

        #[pymethods]
        impl $name {
            #[getter(proto)]
            fn get_proto(&self) -> &str {
                $proto
            }

            #[getter(rsgi_version)]
            fn get_rsgi_version(&self) -> &str {
                RSGI_PROTO_VERSION
            }

            #[getter(http_version)]
            fn get_http_version(&self) -> &str {
                match self.http_version {
                    Version::HTTP_10 => "1",
                    Version::HTTP_11 => "1.1",
                    Version::HTTP_2 => "2",
                    Version::HTTP_3 => "3",
                    _ => "1",
                }
            }

            #[getter(server)]
            fn get_server(&self) -> String {
                self.server.to_string()
            }

            #[getter(client)]
            fn get_client(&self) -> String {
                self.client.to_string()
            }

            #[getter(scheme)]
            fn get_scheme(&self) -> &str {
                self.scheme.as_str()
            }

            #[getter(method)]
            fn get_method(&self) -> &str {
                self.method.as_str()
            }

            #[getter(authority)]
            fn get_authority(&self) -> Option<String> {
                self.uri.authority().map(Authority::to_string)
            }

            #[getter(path)]
            fn get_path(&self) -> Cow<'_, str> {
                percent_decode_str(self.uri.path()).decode_utf8_lossy()
            }

            #[getter(query_string)]
            fn get_query_string(&self) -> &str {
                self.uri.query().unwrap_or("")
            }
        }
    };
}

rsgi_scope_cls!(RSGIHTTPScope, "http");
rsgi_scope_cls!(RSGIWebsocketScope, "ws");

pub(crate) enum PyResponse {
    Body(PyResponseBody),
    File(PyResponseFile),
    FileRange(PyResponseFileRange),
}

pub(crate) struct PyResponseBody {
    status: hyper::StatusCode,
    headers: HeaderMap,
    body: HTTPResponseBody,
}

pub(crate) struct PyResponseFile {
    status: hyper::StatusCode,
    headers: HeaderMap,
    file_path: String,
}

pub(crate) struct PyResponseFileRange {
    status: hyper::StatusCode,
    headers: HeaderMap,
    file_path: String,
    start: u64,
    end: u64,
}

macro_rules! headers_from_py {
    ($headers:expr) => {{
        let mut headers = HeaderMap::with_capacity($headers.len() + 3);
        for (key, value) in $headers {
            headers.append(
                HeaderName::from_bytes(key.as_bytes()).unwrap(),
                HeaderValue::from_str(&value).unwrap(),
            );
        }
        headers.entry(HK_SERVER).or_insert(HV_SERVER);
        headers
    }};
}

impl PyResponseBody {
    pub fn new(status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, body: HTTPResponseBody) -> Self {
        Self {
            status: status.try_into().unwrap(),
            headers: headers_from_py!(headers),
            body,
        }
    }

    pub fn empty(status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>) -> Self {
        Self {
            status: status.try_into().unwrap(),
            headers: headers_from_py!(headers),
            body: empty_body(),
        }
    }

    pub fn from_bytes(status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, body: Box<[u8]>) -> Self {
        Self {
            status: status.try_into().unwrap(),
            headers: headers_from_py!(headers),
            body: http_body_util::Full::new(Bytes::from(body))
                .map_err(std::convert::Into::into)
                .boxed(),
        }
    }

    pub fn from_string(status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, body: String) -> Self {
        Self {
            status: status.try_into().unwrap(),
            headers: headers_from_py!(headers),
            body: http_body_util::Full::new(Bytes::from(body))
                .map_err(std::convert::Into::into)
                .boxed(),
        }
    }

    #[inline]
    pub fn to_response(self) -> hyper::Response<HTTPResponseBody> {
        let mut res = hyper::Response::new(self.body);
        *res.status_mut() = self.status;
        *res.headers_mut() = self.headers;
        res
    }
}

impl PyResponseFile {
    pub fn new(status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, file_path: String) -> Self {
        Self {
            status: status.try_into().unwrap(),
            headers: headers_from_py!(headers),
            file_path,
        }
    }

    #[inline]
    pub async fn to_response(self) -> hyper::Response<HTTPResponseBody> {
        match File::open(&self.file_path).await {
            Ok(file) => {
                let stream = ReaderStream::with_capacity(file, 131_072);
                let stream_body = http_body_util::StreamBody::new(stream.map_ok(hyper::body::Frame::data));
                let mut res = hyper::Response::new(BodyExt::map_err(stream_body, std::convert::Into::into).boxed());
                *res.status_mut() = self.status;
                *res.headers_mut() = self.headers;
                res
            }
            Err(_) => {
                log::info!("Cannot open file {}", &self.file_path);
                response_404()
            }
        }
    }
}

impl PyResponseFileRange {
    pub fn new(status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, file_path: String, start: u64, end: u64) -> Self {
        Self {
            status: status.try_into().unwrap(),
            headers: headers_from_py!(headers),
            file_path,
            start,
            end,
        }
    }

    #[inline]
    pub async fn to_response(self) -> hyper::Response<HTTPResponseBody> {
        use std::io::SeekFrom;
        use tokio::io::{AsyncReadExt, AsyncSeekExt};

        match File::open(&self.file_path).await {
            Ok(mut file) => {
                if file.seek(SeekFrom::Start(self.start)).await.is_err() {
                    log::error!("Cannot seek to position {} in file {}", self.start, &self.file_path);
                    return response_500();
                }
                let take = file.take(self.end - self.start);
                let stream = ReaderStream::with_capacity(take, 131_072);
                let stream_body = http_body_util::StreamBody::new(stream.map_ok(hyper::body::Frame::data));
                let mut res = hyper::Response::new(BodyExt::map_err(stream_body, std::convert::Into::into).boxed());
                *res.status_mut() = self.status;
                *res.headers_mut() = self.headers;
                res
            }
            Err(_) => {
                log::info!("Cannot open file {}", &self.file_path);
                response_404()
            }
        }
    }
}

enum WebsocketMessageType {
    Close = 0,
    Bytes = 1,
    Text = 2,
}

#[pyclass(frozen, module = "granian._granian")]
pub(super) struct WebsocketInboundCloseMessage {
    #[pyo3(get)]
    kind: usize,
}

impl WebsocketInboundCloseMessage {
    pub fn new() -> Self {
        Self {
            kind: WebsocketMessageType::Close as usize,
        }
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(super) struct WebsocketInboundBytesMessage {
    #[pyo3(get)]
    kind: usize,
    #[pyo3(get)]
    data: Py<PyBytes>,
}

impl WebsocketInboundBytesMessage {
    pub fn new(data: Py<PyBytes>) -> Self {
        Self {
            kind: WebsocketMessageType::Bytes as usize,
            data,
        }
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(super) struct WebsocketInboundTextMessage {
    #[pyo3(get)]
    kind: usize,
    #[pyo3(get)]
    data: Py<PyString>,
}

impl WebsocketInboundTextMessage {
    pub fn new(data: Py<PyString>) -> Self {
        Self {
            kind: WebsocketMessageType::Text as usize,
            data,
        }
    }
}
