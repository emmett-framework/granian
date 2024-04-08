use futures::TryStreamExt;
use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use hyper::{
    body::{self, Bytes},
    header,
    http::{request, uri::Authority},
    Version,
};
use percent_encoding::percent_decode_str;
use pyo3::{
    prelude::*,
    types::{IntoPyDict, PyDict},
};
use std::borrow::Cow;
use std::net::SocketAddr;
use tokio::task::JoinHandle;

use super::types::{WSGIBody, WSGIResponseBodyIter};
use crate::callbacks::CallbackWrapper;
use crate::http::empty_body;
use crate::runtime::RuntimeRef;

const WSGI_BYTES_RESPONSE_BODY: i32 = 0;
const WSGI_ITER_RESPONSE_BODY: i32 = 1;

#[inline]
fn run_callback(
    rt: RuntimeRef,
    callback: PyObject,
    mut parts: request::Parts,
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    scheme: &str,
    body: body::Incoming,
) -> PyResult<(i32, Vec<(String, String)>, BoxBody<Bytes, anyhow::Error>)> {
    let (path_raw, query_string) = parts
        .uri
        .path_and_query()
        .map_or_else(|| ("", ""), |pq| (pq.path(), pq.query().unwrap_or("")));
    let path = percent_decode_str(path_raw).decode_utf8().unwrap();
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
    for (key, val) in &parts.headers {
        headers.push((
            format!("HTTP_{}", key.as_str().replace('-', "_").to_uppercase()),
            val.to_str().unwrap_or_default(),
        ));
    }
    if !parts.headers.contains_key(header::HOST) {
        let host = parts.uri.authority().map_or("", Authority::as_str);
        headers.push(("HTTP_HOST".to_string(), host));
    }

    Python::with_gil(|py| {
        let environ = PyDict::new_bound(py);
        environ.set_item(pyo3::intern!(py, "SERVER_PROTOCOL"), version)?;
        environ.set_item(pyo3::intern!(py, "SERVER_NAME"), server.0)?;
        environ.set_item(pyo3::intern!(py, "SERVER_PORT"), server.1)?;
        environ.set_item(pyo3::intern!(py, "REMOTE_ADDR"), client)?;
        environ.set_item(pyo3::intern!(py, "REQUEST_METHOD"), parts.method.as_str())?;
        environ.set_item(pyo3::intern!(py, "PATH_INFO"), path)?;
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

        let (status, headers, body_type, pybody) =
            callback
                .call1(py, (environ,))?
                .extract::<(i32, Vec<(String, String)>, i32, PyObject)>(py)?;
        let body = match body_type {
            WSGI_BYTES_RESPONSE_BODY => {
                let data: Box<[u8]> = pybody.extract::<Cow<[u8]>>(py)?.into();
                http_body_util::Full::new(Bytes::from(data))
                    .map_err(|e| match e {})
                    .boxed()
            }
            WSGI_ITER_RESPONSE_BODY => {
                let body = http_body_util::StreamBody::new(
                    WSGIResponseBodyIter::new(pybody).map_ok(|v| body::Frame::data(Bytes::from(v))),
                );
                BodyExt::boxed(BodyExt::map_err(body, |e| match e {}))
            }
            _ => empty_body(),
        };
        Ok((status, headers, body))
    })
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
) -> JoinHandle<PyResult<(i32, Vec<(String, String)>, BoxBody<Bytes, anyhow::Error>)>> {
    let scheme: std::sync::Arc<str> = scheme.into();
    tokio::task::spawn_blocking(move || run_callback(rt, cb.callback, req, server_addr, client_addr, &scheme, body))
}
