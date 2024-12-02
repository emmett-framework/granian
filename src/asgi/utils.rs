use hyper::{
    header,
    http::{request, uri::Authority},
};
use pyo3::{
    prelude::*,
    sync::GILOnceCell,
    types::{PyBytes, PyDict, PyList, PyString},
};

static ASGI_VERSION: GILOnceCell<PyObject> = GILOnceCell::new();
static ASGI_EXTENSIONS: GILOnceCell<PyObject> = GILOnceCell::new();

macro_rules! scope_native_parts {
    ($req:expr, $server_addr:expr, $client_addr:expr, $path:ident, $query_string:ident, $version:ident, $server:ident, $client:ident) => {
        let (path_raw, $query_string) = $req
            .uri
            .path_and_query()
            .map_or_else(|| ("", ""), |pq| (pq.path(), pq.query().unwrap_or("")));
        let $path = percent_encoding::percent_decode_str(path_raw).decode_utf8().unwrap();
        let $version = match $req.version {
            hyper::Version::HTTP_10 => "1",
            hyper::Version::HTTP_11 => "1.1",
            hyper::Version::HTTP_2 => "2",
            hyper::Version::HTTP_3 => "3",
            _ => "1",
        };
        let $server = ($server_addr.ip().to_string(), $server_addr.port().to_string());
        let $client = ($client_addr.ip().to_string(), $client_addr.port().to_string());
    };
}

#[inline(always)]
pub(super) fn build_scope<'p>(
    py: Python<'p>,
    req: &'p request::Parts,
    proto: &'p str,
    version: &'p str,
    server: (String, String),
    client: (String, String),
    scheme: &'p str,
    path: &'p str,
    query_string: &'p str,
) -> PyResult<Bound<'p, PyDict>> {
    let scope = PyDict::new(py);

    scope.set_item(
        pyo3::intern!(py, "asgi"),
        ASGI_VERSION
            .get_or_try_init(py, || {
                let rv = PyDict::new(py);
                rv.set_item("version", "3.0")?;
                rv.set_item("spec_version", "2.3")?;
                Ok::<PyObject, PyErr>(rv.into())
            })?
            .bind(py),
    )?;
    scope.set_item(
        pyo3::intern!(py, "extensions"),
        ASGI_EXTENSIONS
            .get_or_try_init(py, || {
                let rv = PyDict::new(py);
                rv.set_item("http.response.pathsend", PyDict::new(py))?;
                Ok::<PyObject, PyErr>(rv.into())
            })?
            .bind(py),
    )?;
    scope.set_item(pyo3::intern!(py, "type"), proto)?;
    scope.set_item(pyo3::intern!(py, "http_version"), version)?;
    scope.set_item(pyo3::intern!(py, "server"), server)?;
    scope.set_item(pyo3::intern!(py, "client"), client)?;
    scope.set_item(pyo3::intern!(py, "scheme"), scheme)?;
    scope.set_item(pyo3::intern!(py, "path"), path)?;
    scope.set_item(pyo3::intern!(py, "raw_path"), PyBytes::new(py, path.as_bytes()))?;
    scope.set_item(
        pyo3::intern!(py, "query_string"),
        PyBytes::new(py, query_string.as_bytes()),
    )?;

    let headers = PyList::empty(py);
    for (key, value) in &req.headers {
        headers.append((
            PyBytes::new(py, key.as_str().as_bytes()),
            PyBytes::new(py, value.as_bytes()),
        ))?;
    }
    if !req.headers.contains_key(header::HOST) {
        let host = req.uri.authority().map_or("", Authority::as_str);
        headers.insert(0, (PyBytes::new(py, b"host"), PyBytes::new(py, host.as_bytes())))?;
    }
    scope.set_item(pyo3::intern!(py, "headers"), headers)?;

    Ok(scope)
}

#[inline]
pub(super) fn build_scope_http<'p>(
    py: Python<'p>,
    req: &'p request::Parts,
    version: &'p str,
    server: (String, String),
    client: (String, String),
    scheme: &'p str,
    path: &'p str,
    query_string: &'p str,
) -> PyResult<Bound<'p, PyDict>> {
    let scope = build_scope(py, req, "http", version, server, client, scheme, path, query_string)?;
    scope.set_item(pyo3::intern!(py, "method"), req.method.as_str())?;
    Ok(scope)
}

#[inline]
pub(super) fn build_scope_ws<'p>(
    py: Python<'p>,
    req: &'p request::Parts,
    version: &'p str,
    server: (String, String),
    client: (String, String),
    scheme: &'p str,
    path: &'p str,
    query_string: &'p str,
) -> PyResult<Bound<'p, PyDict>> {
    let scope = build_scope(
        py,
        req,
        "websocket",
        version,
        server,
        client,
        scheme,
        path,
        query_string,
    )?;
    scope.set_item(
        pyo3::intern!(py, "subprotocols"),
        PyList::new(
            py,
            req.headers
                .get_all("Sec-WebSocket-Protocol")
                .iter()
                .map(|v| PyString::new(py, v.to_str().unwrap()))
                .collect::<Vec<Bound<PyString>>>(),
        )?,
    )?;
    Ok(scope)
}

pub(super) use scope_native_parts;
