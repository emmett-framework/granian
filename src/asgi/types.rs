use hyper::{Uri, Version, header::{HeaderMap}};
use pyo3::prelude::*;
use std::collections::HashMap;
use std::net::SocketAddr;


const SCHEME_HTTPS: &str = "https";
const SCHEME_WS: &str = "ws";
const SCHEME_WSS: &str = "wss";

pub(crate) enum ASGIMessageType {
    HTTPStart,
    HTTPBody,
    WSAccept,
    WSClose,
    WSMessage
}

#[pyclass(module="granian._granian")]
pub(crate) struct ASGIScope {
    http_version: Version,
    scheme: String,
    #[pyo3(get)]
    method: String,
    uri: Uri,
    #[pyo3(get)]
    server_ip: String,
    #[pyo3(get)]
    server_port: u16,
    #[pyo3(get)]
    client_ip: String,
    #[pyo3(get)]
    client_port: u16,
    headers: HeaderMap,
    is_websocket: bool
}

// TODO: server address
impl ASGIScope {
    pub fn new(
        http_version: Version,
        scheme: &str,
        uri: Uri,
        method: &str,
        server: SocketAddr,
        client: SocketAddr,
        headers: &HeaderMap
    ) -> Self {
        Self {
            http_version: http_version,
            scheme: scheme.to_string(),
            method: method.to_string(),
            uri: uri,
            server_ip: server.ip().to_string(),
            server_port: server.port(),
            client_ip: client.ip().to_string(),
            client_port: client.port(),
            headers: headers.to_owned(),
            is_websocket: false
        }
    }

    pub fn set_websocket(&mut self) {
        self.is_websocket = true
    }
}

#[pymethods]
impl ASGIScope {
    #[getter(proto)]
    fn get_proto(&self) -> &str {
        match self.is_websocket {
            false => "http",
            true => "websocket"
        }
    }

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
        let scheme = &self.scheme[..];
        match &self.is_websocket {
            false => scheme,
            true => {
                match scheme {
                    SCHEME_HTTPS => SCHEME_WSS,
                    _ => SCHEME_WS
                }
            }
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
