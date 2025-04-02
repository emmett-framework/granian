use hyper::{
    header,
    http::{request, uri::Authority},
};
use pyo3::{
    prelude::*,
    sync::GILOnceCell,
    types::{PyBytes, PyDict, PyList, PyString},
};
use std::net::SocketAddr;

static ASGI_VERSION: GILOnceCell<PyObject> = GILOnceCell::new();
static ASGI_EXTENSIONS: GILOnceCell<PyObject> = GILOnceCell::new();

macro_rules! scope_set {
    ($py:expr, $scope:expr, $key:expr, $val:expr) => {
        $scope.set_item(pyo3::intern!($py, $key), $val)?
    };
}

macro_rules! build_scope_common {
    ($py:expr, $scope:ident, $req:expr, $server:expr, $client:expr, $scheme:expr, $proto:expr) => {
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
                    Ok::<PyObject, PyErr>(rv.into())
                })?
                .bind($py)
        );
        scope_set!(
            $py,
            $scope,
            "extensions",
            ASGI_EXTENSIONS
                .get_or_try_init($py, || {
                    let rv = PyDict::new($py);
                    rv.set_item("http.response.pathsend", PyDict::new($py))?;
                    Ok::<PyObject, PyErr>(rv.into())
                })?
                .bind($py)
        );
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
        scope_set!(
            $py,
            $scope,
            "server",
            ($server.ip().to_string(), $server.port().to_string())
        );
        scope_set!(
            $py,
            $scope,
            "client",
            ($client.ip().to_string(), $client.port().to_string())
        );
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

#[inline]
pub(super) fn build_scope_http<'p>(
    py: Python<'p>,
    req: request::Parts,
    server: SocketAddr,
    client: SocketAddr,
    scheme: &'p str,
) -> PyResult<Bound<'p, PyDict>> {
    build_scope_common!(py, scope, req, server, client, scheme, "http");
    scope_set!(py, scope, "method", req.method.as_str());
    Ok(scope)
}

#[inline]
pub(super) fn build_scope_ws<'p>(
    py: Python<'p>,
    req: request::Parts,
    server: SocketAddr,
    client: SocketAddr,
    scheme: &'p str,
) -> PyResult<Bound<'p, PyDict>> {
    build_scope_common!(py, scope, req, server, client, scheme, "websocket");
    scope_set!(
        py,
        scope,
        "subprotocols",
        PyList::new(
            py,
            req.headers
                .get_all("Sec-WebSocket-Protocol")
                .iter()
                .map(|v| PyString::new(py, v.to_str().unwrap()))
                .collect::<Vec<Bound<PyString>>>(),
        )
        .unwrap()
    );
    Ok(scope)
}
