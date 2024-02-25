use hyper::{
    header::{self, HeaderMap},
    http::uri::Authority,
    Uri, Version,
};
use percent_encoding::percent_decode_str;
use pyo3::{
    prelude::*,
    sync::GILOnceCell,
    types::{PyBytes, PyDict, PyList},
};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

const SCHEME_HTTPS: &str = "https";
const SCHEME_WS: &str = "ws";
const SCHEME_WSS: &str = "wss";

static ASGI_VERSION: GILOnceCell<PyObject> = GILOnceCell::new();
static ASGI_EXTENSIONS: GILOnceCell<PyObject> = GILOnceCell::new();

pub(crate) enum ASGIMessageType {
    HTTPStart,
    HTTPBody,
    HTTPFile,
    WSAccept,
    WSClose,
    WSMessage,
}

macro_rules! asgi_scope_cls {
    ($name:ident, $proto:expr) => {
        #[pyclass(frozen, module = "granian._granian")]
        pub(crate) struct $name {
            http_version: Version,
            scheme: Arc<str>,
            method: Arc<str>,
            uri: Uri,
            server_ip: IpAddr,
            server_port: u16,
            client_ip: IpAddr,
            client_port: u16,
            headers: HeaderMap,
        }

        impl $name {
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
                    scheme: scheme.into(),
                    method: method.into(),
                    uri,
                    server_ip: server.ip(),
                    server_port: server.port(),
                    client_ip: client.ip(),
                    client_port: client.port(),
                    headers: headers.clone(),
                }
            }

            #[inline(always)]
            fn get_proto(&self) -> &str {
                $proto
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
                if !self.headers.contains_key(header::HOST) {
                    let host = self.uri.authority().map_or("", Authority::as_str);
                    rv.insert(0, (PyBytes::new(py, b"host"), PyBytes::new(py, host.as_bytes())))?;
                }
                Ok(rv)
            }
        }
    };
}

asgi_scope_cls!(ASGIHTTPScope, "http");
asgi_scope_cls!(ASGIWebsocketScope, "websocket");

macro_rules! asgi_scope_as_dict {
    ($self:expr, $py:expr, $url_path_prefix:expr, $state:expr, $dict:expr) => {
        let (path, query_string, proto, http_version, server, client) = $py.allow_threads(|| {
            let (path, query_string) = $self
                .uri
                .path_and_query()
                .map_or_else(|| ("", ""), |pq| (pq.path(), pq.query().unwrap_or("")));
            (
                percent_decode_str(path).decode_utf8().unwrap(),
                query_string,
                $self.get_proto(),
                match $self.http_version {
                    Version::HTTP_10 => "1",
                    Version::HTTP_11 => "1.1",
                    Version::HTTP_2 => "2",
                    Version::HTTP_3 => "3",
                    _ => "1",
                },
                ($self.server_ip.to_string(), $self.server_port),
                ($self.client_ip.to_string(), $self.client_port),
            )
        });
        $dict.set_item(
            pyo3::intern!($py, "asgi"),
            ASGI_VERSION
                .get_or_try_init($py, || {
                    let rv = PyDict::new($py);
                    rv.set_item("version", "3.0")?;
                    rv.set_item("spec_version", "2.3")?;
                    Ok::<PyObject, PyErr>(rv.into())
                })?
                .as_ref($py),
        )?;
        $dict.set_item(
            pyo3::intern!($py, "extensions"),
            ASGI_EXTENSIONS
                .get_or_try_init($py, || {
                    let rv = PyDict::new($py);
                    rv.set_item("http.response.pathsend", PyDict::new($py))?;
                    Ok::<PyObject, PyErr>(rv.into())
                })?
                .as_ref($py),
        )?;
        $dict.set_item(pyo3::intern!($py, "type"), proto)?;
        $dict.set_item(pyo3::intern!($py, "http_version"), http_version)?;
        $dict.set_item(pyo3::intern!($py, "server"), server)?;
        $dict.set_item(pyo3::intern!($py, "client"), client)?;
        $dict.set_item(pyo3::intern!($py, "method"), &*$self.method)?;
        $dict.set_item(pyo3::intern!($py, "root_path"), $url_path_prefix)?;
        $dict.set_item(pyo3::intern!($py, "path"), &path)?;
        $dict.set_item(pyo3::intern!($py, "raw_path"), PyBytes::new($py, path.as_bytes()))?;
        $dict.set_item(
            pyo3::intern!($py, "query_string"),
            PyBytes::new($py, query_string.as_bytes()),
        )?;
        $dict.set_item(pyo3::intern!($py, "headers"), $self.py_headers($py)?)?;
        $dict.set_item(pyo3::intern!($py, "state"), $state)?;
    };
}

#[pymethods]
impl ASGIHTTPScope {
    fn as_dict<'p>(&self, py: Python<'p>, url_path_prefix: &'p str, state: &'p PyAny) -> PyResult<&'p PyAny> {
        let dict: &PyDict = PyDict::new(py);
        asgi_scope_as_dict!(self, py, url_path_prefix, state, dict);
        dict.set_item(pyo3::intern!(py, "scheme"), &*self.scheme)?;
        Ok(dict)
    }
}

#[pymethods]
impl ASGIWebsocketScope {
    fn as_dict<'p>(&self, py: Python<'p>, url_path_prefix: &'p str, state: &'p PyAny) -> PyResult<&'p PyAny> {
        let dict: &PyDict = PyDict::new(py);
        asgi_scope_as_dict!(self, py, url_path_prefix, state, dict);
        dict.set_item(
            pyo3::intern!(py, "scheme"),
            match &*self.scheme {
                SCHEME_HTTPS => SCHEME_WSS,
                _ => SCHEME_WS,
            },
        )?;
        Ok(dict)
    }
}
