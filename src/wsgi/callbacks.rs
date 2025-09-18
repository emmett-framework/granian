use hyper::{
    HeaderMap, Version, body, header,
    http::{request, uri::Authority},
};
use itertools::Itertools;
use percent_encoding::percent_decode_str;
use pyo3::{prelude::*, types::PyDict};
use tokio::sync::oneshot;

use super::{io::WSGIProtocol, types::WSGIBody};
use crate::{
    callbacks::ArcCBScheduler,
    http::{HTTPProto, HTTPResponseBody, empty_body},
    net::SockAddr,
    runtime::{Runtime, RuntimeRef},
    utils::log_application_callable_exception,
};

macro_rules! environ_set {
    ($py:expr, $env:expr, $key:expr, $val:expr) => {
        $env.set_item(pyo3::intern!($py, $key), $val)?
    };
}

macro_rules! environ_set_header {
    ($py:expr, $env:expr, $key:expr, $val:expr) => {
        $env.set_item(format!("HTTP_{}", $key.as_str().replace('-', "_").to_uppercase()), $val)?
    };
}

#[inline(always)]
fn build_wsgi(
    py: Python,
    server_addr: SockAddr,
    client_addr: SockAddr,
    scheme: HTTPProto,
    mut req: request::Parts,
    protocol: WSGIProtocol,
    body: WSGIBody,
) -> PyResult<(Py<WSGIProtocol>, Bound<PyDict>)> {
    let (path, query_string) = req.uri.path_and_query().map_or_else(
        || (String::new(), String::new()),
        |pq| {
            (
                encoding_rs::mem::decode_latin1(&percent_decode_str(pq.path()).collect_vec()).into_owned(),
                encoding_rs::mem::decode_latin1(pq.query().unwrap_or("").as_bytes()).into_owned(),
            )
        },
    );
    let proto = Py::new(py, protocol)?;
    let environ = PyDict::new(py);

    environ_set!(
        py,
        environ,
        "SERVER_PROTOCOL",
        match req.version {
            Version::HTTP_10 => "HTTP/1",
            Version::HTTP_11 => "HTTP/1.1",
            Version::HTTP_2 => "HTTP/2",
            Version::HTTP_3 => "HTTP/3",
            _ => "HTTP/1",
        }
    );
    environ_set!(py, environ, "SERVER_NAME", server_addr.ip());
    environ_set!(py, environ, "SERVER_PORT", server_addr.port().to_string());
    environ_set!(py, environ, "REMOTE_ADDR", client_addr.ip());
    environ_set!(py, environ, "REQUEST_METHOD", req.method.as_str());
    environ_set!(py, environ, "PATH_INFO", path);
    environ_set!(py, environ, "QUERY_STRING", query_string);
    environ_set!(py, environ, "wsgi.url_scheme", scheme.as_str());
    environ_set!(py, environ, "wsgi.input", body);

    if let Some(content_type) = req.headers.remove(header::CONTENT_TYPE) {
        environ_set!(py, environ, "CONTENT_TYPE", content_type.to_str().unwrap_or_default());
    }
    if let Some(content_len) = req.headers.remove(header::CONTENT_LENGTH) {
        environ_set!(py, environ, "CONTENT_LENGTH", content_len.to_str().unwrap_or_default());
    }

    // cookies can't be joined by commas
    if let header::Entry::Occupied(h) = req.headers.entry(header::COOKIE) {
        let (hk, hv) = h.remove_entry_mult();
        environ_set_header!(
            py,
            environ,
            hk,
            hv.map(|v| v.to_str().unwrap_or_default().to_owned()).join(";")
        );
    }

    for key in req.headers.keys() {
        environ_set_header!(
            py,
            environ,
            key,
            req.headers
                .get_all(key)
                .iter()
                .map(|v| v.to_str().unwrap_or_default())
                .join(",")
        );
    }
    if !req.headers.contains_key(header::HOST) {
        environ_set!(
            py,
            environ,
            "HTTP_HOST",
            req.uri.authority().map_or("", Authority::as_str)
        );
    }

    Ok((proto, environ))
}

#[inline(always)]
pub(crate) fn call_http(
    rt: RuntimeRef,
    cb: ArcCBScheduler,
    server_addr: SockAddr,
    client_addr: SockAddr,
    scheme: HTTPProto,
    req: request::Parts,
    body: body::Incoming,
) -> oneshot::Receiver<(u16, HeaderMap, HTTPResponseBody)> {
    let (tx, rx) = oneshot::channel();
    let protocol = WSGIProtocol::new(tx);
    let body = WSGIBody::new(rt.clone(), body);

    rt.spawn_blocking(move |py| {
        if let Ok((proto, environ)) = build_wsgi(py, server_addr, client_addr, scheme, req, protocol, body) {
            if let Err(err) = cb.get().cb.call1(py, (proto.clone_ref(py), environ)) {
                log_application_callable_exception(py, &err);
                if let Some(tx) = proto.get().tx() {
                    let _ = tx.send((500, HeaderMap::new(), empty_body()));
                }
            }

            proto.drop_ref(py);
        }
    });

    rx
}
