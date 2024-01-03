use hyper::{header::HeaderMap, Uri, Version};
use once_cell::sync::OnceCell;
use percent_encoding::percent_decode_str;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyString};
use std::net::{IpAddr, SocketAddr};

const SCHEME_HTTPS: &str = "https";
const SCHEME_WS: &str = "ws";
const SCHEME_WSS: &str = "wss";

static ASGI_VERSION: OnceCell<PyObject> = OnceCell::new();
static ASGI_EXTENSIONS: OnceCell<PyObject> = OnceCell::new();

pub(crate) enum ASGIMessageType {
    HTTPStart,
    HTTPBody,
    WSAccept,
    WSClose,
    WSMessage,
}

#[pyclass(module = "granian._granian")]
pub(crate) struct ASGIScope {
    http_version: Version,
    scheme: String,
    method: String,
    uri: Uri,
    server_ip: IpAddr,
    server_port: u16,
    client_ip: IpAddr,
    client_port: u16,
    headers: HeaderMap,
    is_websocket: bool,
}

impl ASGIScope {
    pub fn new(
        http_version: Version,
        scheme: &str,
        uri: Uri,
        method: &str,
        server: SocketAddr,
        client: SocketAddr,
        headers: &HeaderMap,
    ) -> Self {
        Self {
            http_version,
            scheme: scheme.to_string(),
            method: method.to_string(),
            uri,
            server_ip: server.ip(),
            server_port: server.port(),
            client_ip: client.ip(),
            client_port: client.port(),
            headers: headers.clone(),
            is_websocket: false,
        }
    }

    pub fn set_websocket(&mut self) {
        self.is_websocket = true;
    }

    #[inline(always)]
    fn py_proto(&self) -> &str {
        match self.is_websocket {
            false => "http",
            true => "websocket",
        }
    }

    #[inline(always)]
    fn py_http_version(&self) -> &str {
        match self.http_version {
            Version::HTTP_10 => "1",
            Version::HTTP_11 => "1.1",
            Version::HTTP_2 => "2",
            Version::HTTP_3 => "3",
            _ => "1",
        }
    }

    #[inline(always)]
    fn py_scheme(&self) -> &str {
        let scheme = &self.scheme[..];
        match self.is_websocket {
            false => scheme,
            true => match scheme {
                SCHEME_HTTPS => SCHEME_WSS,
                _ => SCHEME_WS,
            },
        }
    }

    #[inline(always)]
    fn py_headers<'p>(&self, py: Python<'p>) -> PyResult<&'p PyList> {
        let rv = PyList::empty(py);
        for (key, value) in &self.headers {
            rv.append((
                PyBytes::new(py, key.as_str().as_bytes()),
                PyBytes::new(py, value.as_bytes()),
            ))?;
        }
        Ok(rv)
    }
}

#[pymethods]
impl ASGIScope {
    fn as_dict<'p>(&self, py: Python<'p>, url_path_prefix: &'p str) -> PyResult<&'p PyAny> {
        let (path, query_string, proto, http_version, server, client, scheme, method) = py.allow_threads(|| {
            let (path, query_string) = self
                .uri
                .path_and_query()
                .map_or_else(|| ("", ""), |pq| (pq.path(), pq.query().unwrap_or("")));
            (
                percent_decode_str(path).decode_utf8().unwrap(),
                query_string,
                self.py_proto(),
                self.py_http_version(),
                (self.server_ip.to_string(), self.server_port),
                (self.client_ip.to_string(), self.client_port),
                self.py_scheme(),
                &self.method[..],
            )
        });
        let dict: &PyDict = PyDict::new(py);
        dict.set_item(
            pyo3::intern!(py, "asgi"),
            ASGI_VERSION
                .get_or_try_init(|| {
                    let rv = PyDict::new(py);
                    rv.set_item("version", "3.0")?;
                    rv.set_item("spec_version", "2.3")?;
                    Ok::<PyObject, PyErr>(rv.into())
                })?
                .as_ref(py),
        )?;
        dict.set_item(
            pyo3::intern!(py, "extensions"),
            ASGI_EXTENSIONS
                .get_or_try_init(|| {
                    let rv = PyDict::new(py);
                    Ok::<PyObject, PyErr>(rv.into())
                })?
                .as_ref(py),
        )?;
        dict.set_item(pyo3::intern!(py, "type"), proto)?;
        dict.set_item(pyo3::intern!(py, "http_version"), http_version)?;
        dict.set_item(pyo3::intern!(py, "server"), server)?;
        dict.set_item(pyo3::intern!(py, "client"), client)?;
        dict.set_item(pyo3::intern!(py, "scheme"), scheme)?;
        dict.set_item(pyo3::intern!(py, "method"), method)?;
        dict.set_item(pyo3::intern!(py, "root_path"), url_path_prefix)?;
        dict.set_item(pyo3::intern!(py, "path"), &path)?;
        dict.set_item(
            pyo3::intern!(py, "raw_path"),
            PyString::new(py, &path).call_method1(pyo3::intern!(py, "encode"), (pyo3::intern!(py, "ascii"),))?,
        )?;
        dict.set_item(
            pyo3::intern!(py, "query_string"),
            PyString::new(py, query_string)
                .call_method1(pyo3::intern!(py, "encode"), (pyo3::intern!(py, "latin-1"),))?,
        )?;
        dict.set_item(pyo3::intern!(py, "headers"), self.py_headers(py)?)?;
        Ok(dict)
    }
}
