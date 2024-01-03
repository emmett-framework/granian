use bytes::Bytes;
use hyper::{
    header::{HeaderMap, HeaderName, HeaderValue, SERVER as HK_SERVER},
    Body, Uri, Version,
};
use percent_encoding::percent_decode_str;
use pyo3::prelude::*;
use pyo3::types::PyString;
use std::{borrow::Cow, net::SocketAddr};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::http::HV_SERVER;

#[pyclass(module = "granian._granian")]
#[derive(Clone)]
pub(crate) struct RSGIHeaders {
    inner: HeaderMap,
}

impl RSGIHeaders {
    pub fn new(map: &HeaderMap) -> Self {
        Self { inner: map.clone() }
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

    fn values(&self) -> PyResult<Vec<&str>> {
        let mut ret = Vec::with_capacity(self.inner.keys_len());
        for val in self.inner.values() {
            ret.push(val.to_str().unwrap());
        }
        Ok(ret)
    }

    fn items(&self) -> PyResult<Vec<(&str, &str)>> {
        let mut ret = Vec::with_capacity(self.inner.keys_len());
        for (key, val) in &self.inner {
            ret.push((key.as_str(), val.to_str().unwrap()));
        }
        Ok(ret)
    }

    fn __contains__(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    #[pyo3(signature = (key, default=None))]
    fn get(&self, py: Python, key: &str, default: Option<PyObject>) -> Option<PyObject> {
        match self.inner.get(key) {
            Some(val) => match val.to_str() {
                Ok(string) => Some(PyString::new(py, string).into()),
                _ => default,
            },
            _ => default,
        }
    }
}

#[pyclass(module = "granian._granian")]
pub(crate) struct RSGIScope {
    #[pyo3(get)]
    proto: String,
    http_version: Version,
    #[pyo3(get)]
    rsgi_version: String,
    #[pyo3(get)]
    scheme: String,
    #[pyo3(get)]
    method: String,
    uri: Uri,
    #[pyo3(get)]
    server: String,
    #[pyo3(get)]
    client: String,
    #[pyo3(get)]
    headers: RSGIHeaders,
}

impl RSGIScope {
    pub fn new(
        proto: &str,
        http_version: Version,
        scheme: &str,
        uri: Uri,
        method: &str,
        server: SocketAddr,
        client: SocketAddr,
        headers: &HeaderMap,
    ) -> Self {
        Self {
            proto: proto.to_string(),
            http_version,
            rsgi_version: "1.2".to_string(),
            scheme: scheme.to_string(),
            method: method.to_string(),
            uri,
            server: server.to_string(),
            client: client.to_string(),
            headers: RSGIHeaders::new(headers),
        }
    }

    pub fn set_proto(&mut self, value: &str) {
        self.proto = value.to_string();
    }
}

#[pymethods]
impl RSGIScope {
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

    #[getter(path)]
    fn get_path(&self) -> Cow<str> {
        percent_decode_str(self.uri.path()).decode_utf8().unwrap()
    }

    #[getter(query_string)]
    fn get_query_string(&self) -> &str {
        self.uri.query().unwrap_or("")
    }
}

pub(crate) enum PyResponse {
    Body(PyResponseBody),
    File(PyResponseFile),
}

pub(crate) struct PyResponseBody {
    status: u16,
    headers: Vec<(String, String)>,
    body: Body,
}

pub(crate) struct PyResponseFile {
    status: u16,
    headers: Vec<(String, String)>,
    file_path: String,
}

macro_rules! response_head_from_py {
    ($status:expr, $headers:expr, $res:expr) => {{
        let mut rh = hyper::http::HeaderMap::new();

        rh.insert(HK_SERVER, HV_SERVER);
        for (key, value) in $headers {
            rh.append(
                HeaderName::from_bytes(key.as_bytes()).unwrap(),
                HeaderValue::from_str(&value).unwrap(),
            );
        }

        *$res.status_mut() = $status.try_into().unwrap();
        *$res.headers_mut() = rh;
    }};
}

impl PyResponseBody {
    pub fn new(status: u16, headers: Vec<(String, String)>, body: Body) -> Self {
        Self { status, headers, body }
    }

    pub fn empty(status: u16, headers: Vec<(String, String)>) -> Self {
        Self {
            status,
            headers,
            body: Body::empty(),
        }
    }

    pub fn from_bytes(status: u16, headers: Vec<(String, String)>, body: Cow<[u8]>) -> Self {
        let rbody: Box<[u8]> = body.into();
        Self {
            status,
            headers,
            body: Body::from(Bytes::from(rbody)),
        }
    }

    pub fn from_string(status: u16, headers: Vec<(String, String)>, body: String) -> Self {
        Self {
            status,
            headers,
            body: Body::from(body),
        }
    }

    pub fn to_response(self) -> hyper::Response<Body> {
        let mut res = hyper::Response::<Body>::new(self.body);
        response_head_from_py!(self.status, &self.headers, res);
        res
    }
}

impl PyResponseFile {
    pub fn new(status: u16, headers: Vec<(String, String)>, file_path: String) -> Self {
        Self {
            status,
            headers,
            file_path,
        }
    }

    pub async fn to_response(&self) -> hyper::Response<Body> {
        let file = File::open(&self.file_path).await.unwrap();
        let stream = FramedRead::new(file, BytesCodec::new());
        let mut res = hyper::Response::<Body>::new(Body::wrap_stream(stream));
        response_head_from_py!(self.status, &self.headers, res);
        res
    }
}
