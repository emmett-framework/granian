use hyper::{Uri, Version, header::{HeaderMap}};
use pyo3::prelude::*;
use pyo3::types::{PyString};
use std::net::SocketAddr;

#[pyclass(module="granian.rsgi")]
#[derive(Clone)]
pub(crate) struct Headers {
    inner: HeaderMap
}

impl Headers {
    pub fn new(map: &HeaderMap) -> Self {
        Self { inner: map.clone() }
    }
}

#[pymethods]
impl Headers {
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

#[pyclass(module="granian.rsgi")]
pub(crate) struct Scope {
    #[pyo3(get)]
    proto: String,
    http_version: Version,
    #[pyo3(get)]
    method: String,
    uri: Uri,
    #[pyo3(get)]
    client: String,
    #[pyo3(get)]
    headers: Headers
}

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
            headers: Headers::new(headers)
        }
    }
}

#[pymethods]
impl Scope {
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

pub(crate) enum ResponseType {
    Bytes = 1,
    String = 2,
    FilePath = 10,
    // Chunks = 20,
    // AsyncIter = 30
}
