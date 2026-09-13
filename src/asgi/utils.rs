use hyper::{
    header,
    http::{request, uri::Authority},
};
use pyo3::{
    prelude::*,
    sync::PyOnceLock,
    types::{PyBytes, PyDict, PyList, PyString},
};

use crate::{http::HTTPProto, net::SockAddr};

static ASGI_VERSION: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
static ASGI_EXTENSIONS: PyOnceLock<Py<PyAny>> = PyOnceLock::new();

macro_rules! scope_set {
    ($py:expr, $scope:expr, $key:expr, $val:expr) => {
        $scope.set_item(pyo3::intern!($py, $key), $val)?
    };
}

macro_rules! build_scope_common {
    ($py:expr, $scope:ident, $req:expr, $server:expr, $client:expr, $scheme:expr, $proto:expr, $tls:expr) => {
        let raw_path = $req.uri.path();
        let query_string = $req.uri.query().unwrap_or("");
        let path = percent_encoding::percent_decode_str(raw_path).decode_utf8_lossy();
        let $scope = PyDict::new($py);

        scope_set!(
            $py,
            $scope,
            "asgi",
            ASGI_VERSION
                .get_or_try_init($py, || {
                    let rv = PyDict::new($py);
                    rv.set_item("version", "3.0")?;
                    rv.set_item("spec_version", "2.3")?;
                    Ok::<Py<PyAny>, PyErr>(rv.into())
                })?
                .bind($py)
        );
        // For plain connections the cached singleton extensions dict is reused
        // (zero allocation). TLS connections get a fresh dict carrying the
        // standard ASGI `tls` extension alongside the static extension keys.
        match &$tls {
            Some(info) => {
                let ext = PyDict::new($py);
                ext.set_item(pyo3::intern!($py, "http.response.pathsend"), PyDict::new($py))?;
                ext.set_item(pyo3::intern!($py, "websocket.http.response"), PyDict::new($py))?;
                ext.set_item(pyo3::intern!($py, "tls"), tls_scope_dict($py, info)?)?;
                scope_set!($py, $scope, "extensions", ext);
            }
            None => {
                scope_set!(
                    $py,
                    $scope,
                    "extensions",
                    ASGI_EXTENSIONS
                        .get_or_try_init($py, || {
                            let rv = PyDict::new($py);
                            rv.set_item("http.response.pathsend", PyDict::new($py))?;
                            rv.set_item("websocket.http.response", PyDict::new($py))?;
                            Ok::<Py<PyAny>, PyErr>(rv.into())
                        })?
                        .bind($py)
                );
            }
        }
        scope_set!($py, $scope, "type", $proto);
        scope_set!(
            $py,
            $scope,
            "http_version",
            match $req.version {
                hyper::Version::HTTP_10 => "1",
                hyper::Version::HTTP_11 => "1.1",
                hyper::Version::HTTP_2 => "2",
                hyper::Version::HTTP_3 => "3",
                _ => "1",
            }
        );
        scope_set!($py, $scope, "server", ($server.ip(), $server.port().to_string()));
        scope_set!($py, $scope, "client", ($client.ip(), $client.port().to_string()));
        scope_set!($py, $scope, "scheme", $scheme);
        scope_set!($py, $scope, "path", &path);
        scope_set!($py, $scope, "raw_path", PyBytes::new($py, raw_path.as_bytes()));
        scope_set!($py, $scope, "query_string", PyBytes::new($py, query_string.as_bytes()));

        let headers = PyList::empty($py);
        for (key, value) in &$req.headers {
            headers.append((
                PyBytes::new($py, key.as_str().as_bytes()),
                PyBytes::new($py, value.as_bytes()),
            ))?;
        }
        if !$req.headers.contains_key(header::HOST) {
            let host = $req.uri.authority().map_or("", Authority::as_str);
            headers.insert(0, (PyBytes::new($py, b"host"), PyBytes::new($py, host.as_bytes())))?;
        }
        scope_set!($py, $scope, "headers", headers);
    };
}

/// Builds the `scope["extensions"]["tls"]` dict per the ASGI TLS extension
/// (<https://asgi.readthedocs.io/en/latest/specs/tls.html>).
#[inline]
fn tls_scope_dict<'p>(py: Python<'p>, info: &crate::tls::TlsSessionInfo) -> PyResult<Bound<'p, PyDict>> {
    let tls = PyDict::new(py);
    tls.set_item(pyo3::intern!(py, "server_cert"), py.None())?;
    tls.set_item(
        pyo3::intern!(py, "client_cert_chain"),
        PyList::new(py, info.client_cert_chain.iter().map(|pem| PyString::new(py, pem)))?,
    )?;
    tls.set_item(pyo3::intern!(py, "client_cert_name"), py.None())?;
    tls.set_item(pyo3::intern!(py, "client_cert_error"), py.None())?;
    tls.set_item(pyo3::intern!(py, "tls_version"), info.tls_version)?;
    tls.set_item(pyo3::intern!(py, "cipher_suite"), info.cipher_suite)?;
    Ok(tls)
}

#[inline]
pub(super) fn build_scope_http(
    py: Python,
    req: request::Parts,
    server: SockAddr,
    client: SockAddr,
    scheme: HTTPProto,
    tls: crate::tls::TlsCtx,
) -> PyResult<Bound<PyDict>> {
    build_scope_common!(py, scope, req, server, client, scheme.as_str(), "http", tls);
    scope_set!(py, scope, "method", req.method.as_str());
    Ok(scope)
}

#[inline]
pub(super) fn build_scope_ws(
    py: Python,
    req: request::Parts,
    server: SockAddr,
    client: SockAddr,
    scheme: HTTPProto,
    tls: crate::tls::TlsCtx,
) -> PyResult<Bound<PyDict>> {
    let ws_scheme = match scheme {
        HTTPProto::Plain => "ws",
        HTTPProto::Tls => "wss",
    };
    build_scope_common!(py, scope, req, server, client, ws_scheme, "websocket", tls);
    scope_set!(
        py,
        scope,
        "subprotocols",
        PyList::new(
            py,
            req.headers
                .get_all("Sec-WebSocket-Protocol")
                .iter()
                .flat_map(|v| {
                    v.to_str()
                        .unwrap_or_default()
                        .split(',')
                        .filter(|s| !s.is_empty())
                        .map(|s| PyString::new(py, s.trim()))
                })
                .collect::<Vec<Bound<PyString>>>(),
        )
        .unwrap()
    );
    Ok(scope)
}
