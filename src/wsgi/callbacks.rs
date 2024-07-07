use hyper::{
    body, header,
    http::{request, uri::Authority},
    HeaderMap, Version,
};
use itertools::Itertools;
use percent_encoding::percent_decode_str;
use pyo3::{
    prelude::*,
    types::{IntoPyDict, PyBytes, PyDict},
};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::oneshot;

use super::{io::WSGIProtocol, types::WSGIBody};
use crate::{
    callbacks::CallbackWrapper,
    http::{empty_body, HTTPResponseBody},
    runtime::RuntimeRef,
    utils::log_application_callable_exception,
};

#[inline]
fn run_callback(
    rt: RuntimeRef,
    tx: oneshot::Sender<(u16, HeaderMap, HTTPResponseBody)>,
    callback: Arc<PyObject>,
    mut parts: request::Parts,
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    scheme: &str,
    body: body::Incoming,
) {
    let (path_raw, query_string) = parts
        .uri
        .path_and_query()
        .map_or_else(|| ("", ""), |pq| (pq.path(), pq.query().unwrap_or("")));
    let path = percent_decode_str(path_raw).collect_vec();
    let version = match parts.version {
        Version::HTTP_10 => "HTTP/1",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2",
        Version::HTTP_3 => "HTTP/3",
        _ => "HTTP/1",
    };
    let server = (server_addr.ip().to_string(), server_addr.port().to_string());
    let client = client_addr.to_string();
    let content_type = parts.headers.remove(header::CONTENT_TYPE);
    let content_len = parts.headers.remove(header::CONTENT_LENGTH);
    let mut headers = Vec::with_capacity(parts.headers.len());
    for key in parts.headers.keys() {
        headers.push((
            format!("HTTP_{}", key.as_str().replace('-', "_").to_uppercase()),
            parts
                .headers
                .get_all(key)
                .iter()
                .map(|v| v.to_str().unwrap_or_default())
                .join(","),
        ));
    }
    if !parts.headers.contains_key(header::HOST) {
        let host = parts.uri.authority().map_or("", Authority::as_str);
        headers.push(("HTTP_HOST".to_string(), host.to_string()));
    }

    let _ = Python::with_gil(|py| -> PyResult<()> {
        let proto = Py::new(py, WSGIProtocol::new(tx))?;
        let callback = callback.clone_ref(py);
        let environ = PyDict::new_bound(py);
        environ.set_item(pyo3::intern!(py, "SERVER_PROTOCOL"), version)?;
        environ.set_item(pyo3::intern!(py, "SERVER_NAME"), server.0)?;
        environ.set_item(pyo3::intern!(py, "SERVER_PORT"), server.1)?;
        environ.set_item(pyo3::intern!(py, "REMOTE_ADDR"), client)?;
        environ.set_item(pyo3::intern!(py, "REQUEST_METHOD"), parts.method.as_str())?;
        environ.set_item(
            pyo3::intern!(py, "PATH_INFO"),
            PyBytes::new_bound(py, &path).call_method1(pyo3::intern!(py, "decode"), (pyo3::intern!(py, "latin1"),))?,
        )?;
        environ.set_item(pyo3::intern!(py, "QUERY_STRING"), query_string)?;
        environ.set_item(pyo3::intern!(py, "wsgi.url_scheme"), scheme)?;
        environ.set_item(pyo3::intern!(py, "wsgi.input"), Py::new(py, WSGIBody::new(rt, body))?)?;
        if let Some(content_type) = content_type {
            environ.set_item(
                pyo3::intern!(py, "CONTENT_TYPE"),
                content_type.to_str().unwrap_or_default(),
            )?;
        }
        if let Some(content_len) = content_len {
            environ.set_item(
                pyo3::intern!(py, "CONTENT_LENGTH"),
                content_len.to_str().unwrap_or_default(),
            )?;
        }
        environ.update(headers.into_py_dict_bound(py).as_mapping())?;

        if let Err(err) = callback.call1(py, (proto.clone_ref(py), environ)) {
            log_application_callable_exception(&err);
            if let Some(tx) = proto.get().tx() {
                let _ = tx.send((500, HeaderMap::new(), empty_body()));
            }
        }

        Ok(())
    });
}

#[inline(always)]
pub(crate) fn call_http(
    rt: RuntimeRef,
    cb: CallbackWrapper,
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    scheme: &str,
    req: request::Parts,
    body: body::Incoming,
) -> oneshot::Receiver<(u16, HeaderMap, HTTPResponseBody)> {
    let scheme: std::sync::Arc<str> = scheme.into();
    let (tx, rx) = oneshot::channel();
    tokio::task::spawn_blocking(move || {
        run_callback(rt, tx, cb.callback, req, server_addr, client_addr, &scheme, body);
    });
    rx
}
