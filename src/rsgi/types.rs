use hyper::{
    header::{HeaderMap, HeaderName, HeaderValue, SERVER as HK_SERVER},
    http::response::Builder as ResponseBuilder, Body, Uri, Version
};
use pyo3::prelude::*;
use pyo3::types::{PyString};
use std::net::SocketAddr;

use crate::http::HV_SERVER;


#[pyclass(module="granian._granian")]
#[derive(Clone)]
pub(crate) struct RSGIHeaders {
    inner: HeaderMap
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
        };
        ret
    }

    fn values(&self) -> PyResult<Vec<&str>> {
        let mut ret = Vec::with_capacity(self.inner.keys_len());
        for val in self.inner.values() {
            ret.push(val.to_str().unwrap());
        };
        Ok(ret)
    }

    fn items(&self) -> PyResult<Vec<(&str, &str)>> {
        let mut ret = Vec::with_capacity(self.inner.keys_len());
        for (key, val) in self.inner.iter() {
            ret.push((key.as_str(), val.to_str().unwrap()));
        };
        Ok(ret)
    }

    fn __contains__(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    #[args(key, default="None")]
    fn get(&self, py: Python, key: &str, default: Option<PyObject>) -> Option<PyObject> {
        match self.inner.get(key) {
            Some(val) => {
                match val.to_str() {
                    Ok(string) => Some(PyString::new(py, string).into()),
                    _ => default
                }
            },
            _ => default
        }
    }
}

#[pyclass(module="granian._granian")]
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
    headers: RSGIHeaders
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
        headers: &HeaderMap
    ) -> Self {
        Self {
            proto: proto.to_string(),
            http_version: http_version,
            rsgi_version: "1.0".to_string(),
            scheme: scheme.to_string(),
            method: method.to_string(),
            uri: uri,
            server: server.to_string(),
            client: client.to_string(),
            headers: RSGIHeaders::new(headers)
        }
    }

    pub fn set_proto(&mut self, value: &str) {
        self.proto = value.to_string()
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
            _ => "1"
        }
    }

    #[getter(path)]
    fn get_path(&self) -> &str {
        self.uri.path()
    }

    #[getter(query_string)]
    fn get_query_string(&self) -> &str {
        self.uri.query().unwrap_or("")
    }
}

#[derive(Debug)]
pub(crate) enum ResponseType {
    Body = 1,
    File = 10
}

#[pyclass(frozen)]
#[derive(Debug)]
pub(crate) struct Response {
    pub inner: ResponseBuilder,
    pub mode: ResponseType,
    pub body: Body,
    pub file: Option<String>
}

impl Response {
    pub fn new() -> Self {
        Self {
            inner: ResponseBuilder::new().status(200),
            mode: ResponseType::Body,
            body: Body::empty(),
            file: None
        }
    }

    pub fn head(&mut self, status: u16, headers: &Vec<(&str, &str)>) {
        match status {
            200 => {},
            _ => {
                self.inner = ResponseBuilder::new().status(status);
            }
        }

        let rh = self.inner.headers_mut().unwrap();
        rh.insert(HK_SERVER, HV_SERVER);
        for (key, value) in headers {
            rh.append(
                HeaderName::from_bytes(key.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap()
            );
        }
    }

    pub fn error(&mut self) {
        self.inner = ResponseBuilder::new().status(500);
        self.body = Body::from("Internal server error");
    }
}
