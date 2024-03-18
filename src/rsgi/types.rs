use anyhow::Result;
use futures::TryStreamExt;
use http_body_util::BodyExt;
use hyper::{
    body::Bytes,
    header::{HeaderMap, HeaderName, HeaderValue, SERVER as HK_SERVER},
    http::uri::Authority,
    Method, Uri, Version,
};
use percent_encoding::percent_decode_str;
use pyo3::prelude::*;
use pyo3::types::{PyIterator, PyList, PyString};
use std::{borrow::Cow, net::SocketAddr, sync::Arc};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::http::{empty_body, response_404, HTTPResponseBody, HV_SERVER};

const RSGI_PROTO_VERSION: &str = "1.4";

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

    fn values(&self) -> Result<Vec<&str>> {
        let mut ret = Vec::with_capacity(self.inner.keys_len());
        for val in self.inner.values() {
            ret.push(val.to_str()?);
        }
        Ok(ret)
    }

    fn items(&self) -> Result<Vec<(&str, &str)>> {
        let mut ret = Vec::with_capacity(self.inner.keys_len());
        for (key, val) in &self.inner {
            ret.push((key.as_str(), val.to_str()?));
        }
        Ok(ret)
    }

    fn __contains__(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    fn __getitem__(&self, key: &str) -> Result<&str> {
        match self.inner.get(key) {
            Some(value) => Ok(value.to_str()?),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(key.to_owned()).into()),
        }
    }

    fn __iter__<'p>(&self, py: Python<'p>) -> PyResult<&'p PyIterator> {
        PyIterator::from_object(PyList::new(py, self.keys()))
    }

    fn __len__(&self) -> usize {
        self.inner.keys_len()
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

    #[pyo3(signature = (key))]
    fn get_all<'p>(&self, py: Python<'p>, key: &'p str) -> &'p PyList {
        PyList::new(
            py,
            self.inner
                .get_all(key)
                .iter()
                .map(|v| PyString::new(py, v.to_str().unwrap()))
                .collect::<Vec<&PyString>>(),
        )
    }
}

macro_rules! rsgi_scope_cls {
    ($name:ident, $proto:expr) => {
        #[pyclass(frozen, module = "granian._granian")]
        pub(crate) struct $name {
            http_version: Version,
            scheme: Arc<str>,
            method: Method,
            uri: Uri,
            #[pyo3(get)]
            server: String,
            #[pyo3(get)]
            client: String,
            #[pyo3(get)]
            headers: RSGIHeaders,
        }

        impl $name {
            pub fn new(
                http_version: Version,
                scheme: &str,
                uri: Uri,
                method: Method,
                server: SocketAddr,
                client: SocketAddr,
                headers: HeaderMap,
            ) -> Self {
                Self {
                    http_version,
                    scheme: scheme.into(),
                    method,
                    uri,
                    server: server.to_string(),
                    client: client.to_string(),
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

            #[getter(scheme)]
            fn get_scheme(&self) -> &str {
                &self.scheme
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
            fn get_path(&self) -> Result<Cow<str>> {
                Ok(percent_decode_str(self.uri.path()).decode_utf8()?)
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
}

pub(crate) struct PyResponseBody {
    status: u16,
    headers: Vec<(String, String)>,
    body: HTTPResponseBody,
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
    pub fn new(status: u16, headers: Vec<(String, String)>, body: HTTPResponseBody) -> Self {
        Self { status, headers, body }
    }

    pub fn empty(status: u16, headers: Vec<(String, String)>) -> Self {
        Self {
            status,
            headers,
            body: empty_body(),
        }
    }

    pub fn from_bytes(status: u16, headers: Vec<(String, String)>, body: Cow<[u8]>) -> Self {
        let rbody: Box<[u8]> = body.into();
        Self {
            status,
            headers,
            body: http_body_util::Full::new(Bytes::from(rbody))
                .map_err(|e| match e {})
                .boxed(),
        }
    }

    pub fn from_string(status: u16, headers: Vec<(String, String)>, body: String) -> Self {
        Self {
            status,
            headers,
            body: http_body_util::Full::new(Bytes::from(body))
                .map_err(|e| match e {})
                .boxed(),
        }
    }

    #[inline]
    pub fn to_response(self) -> hyper::Response<HTTPResponseBody> {
        let mut res = hyper::Response::new(self.body);
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

    pub async fn to_response(&self) -> hyper::Response<HTTPResponseBody> {
        match File::open(&self.file_path).await {
            Ok(file) => {
                let stream = ReaderStream::new(file);
                let stream_body = http_body_util::StreamBody::new(stream.map_ok(hyper::body::Frame::data));
                let mut res = hyper::Response::new(BodyExt::map_err(stream_body, std::convert::Into::into).boxed());
                response_head_from_py!(self.status, &self.headers, res);
                res
            }
            Err(_) => {
                log::info!("Cannot open file {}", &self.file_path);
                response_404()
            }
        }
    }
}
