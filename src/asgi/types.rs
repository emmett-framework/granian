use hyper::{Uri, Version, header::{HeaderMap}};
use pyo3::prelude::*;
use std::collections::HashMap;
use std::net::SocketAddr;

pub(crate) enum ASGIMessageType {
    Start = 0,
    Body = 1
}

#[pyclass(module="granian.asgi")]
pub(crate) struct Scope {
    #[pyo3(get)]
    proto: String,
    http_version: Version,
    #[pyo3(get)]
    method: String,
    uri: Uri,
    #[pyo3(get)]
    client: String,
    headers: HeaderMap
}

// TODO: server address
impl Scope {
    pub fn new(
        proto: &str,
        http_version: Version,
        uri: Uri,
        method: &str,
        client: SocketAddr,
        headers: &HeaderMap
    ) -> Self {
        Self {
            proto: proto.to_string(),
            http_version: http_version,
            method: method.to_string(),
            uri: uri,
            client: client.to_string(),
            headers: headers.to_owned()
        }
    }
}

#[pymethods]
impl Scope {
    #[getter(headers)]
    fn get_headers(&self) -> HashMap<&[u8], &[u8]> {
        let mut ret = HashMap::new();
        for (key, value) in self.headers.iter() {
            ret.insert(key.as_str().as_bytes(), value.as_bytes());
        }
        ret
    }

    #[getter(http_version)]
    fn get_http_version(&self) -> &str {
        match self.http_version {
            Version::HTTP_10 => "1",
            Version::HTTP_11 => "1.1",
            Version::HTTP_2 => "2",
            _ => "1"
        }
    }

    #[getter(scheme)]
    fn get_scheme(&self) -> &str {
        self.uri.scheme_str().unwrap_or("http")
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
