use hyper::{
    header::{CONTENT_LENGTH, CONTENT_TYPE, HOST},
    http::uri::Authority,
    Version,
};
use percent_encoding::percent_decode_str;
use pyo3::types::PyDict;
use pyo3::{prelude::*, types::IntoPyDict};
use std::net::SocketAddr;

use super::types::WSGIBody;
use crate::runtime::RuntimeRef;

#[inline]
pub(crate) fn build_environ<'p>(
    py: Python<'p>,
    rt: RuntimeRef,
    mut req: hyper::http::request::Parts,
    body: hyper::body::Incoming,
    server: SocketAddr,
    client: SocketAddr,
    scheme: &str,
) -> PyResult<&'p PyDict> {
    let (path, query_string, http_version, server, client, content_type, content_len, headers, body) = py
        .allow_threads(|| {
            let (path, query_string) = req
                .uri
                .path_and_query()
                .map_or_else(|| ("", ""), |pq| (pq.path(), pq.query().unwrap_or("")));
            let content_type = req.headers.remove(CONTENT_TYPE);
            let content_len = req.headers.remove(CONTENT_LENGTH);
            let mut headers = Vec::with_capacity(req.headers.len());

            for (key, val) in &req.headers {
                headers.push((
                    format!("HTTP_{}", key.as_str().replace('-', "_").to_uppercase()),
                    val.to_str().unwrap_or_default().to_owned(),
                ));
            }
            if !req.headers.contains_key(HOST) {
                let host = req.uri.authority().map_or("", Authority::as_str);
                headers.push(("HTTP_HOST".to_string(), host.to_owned()));
            }

            (
                percent_decode_str(path).decode_utf8().unwrap(),
                query_string,
                match req.version {
                    Version::HTTP_10 => "HTTP/1",
                    Version::HTTP_11 => "HTTP/1.1",
                    Version::HTTP_2 => "HTTP/2",
                    Version::HTTP_3 => "HTTP/3",
                    _ => "HTTP/1",
                },
                (server.ip().to_string(), server.port().to_string()),
                client.to_string(),
                content_type,
                content_len,
                headers,
                WSGIBody::new(rt, body),
            )
        });

    let ret: &PyDict = PyDict::new(py);
    ret.set_item(pyo3::intern!(py, "SERVER_PROTOCOL"), http_version)?;
    ret.set_item(pyo3::intern!(py, "SERVER_NAME"), server.0)?;
    ret.set_item(pyo3::intern!(py, "SERVER_PORT"), server.1)?;
    ret.set_item(pyo3::intern!(py, "REMOTE_ADDR"), client)?;
    ret.set_item(pyo3::intern!(py, "REQUEST_METHOD"), req.method.as_str())?;
    ret.set_item(pyo3::intern!(py, "PATH_INFO"), path)?;
    ret.set_item(pyo3::intern!(py, "QUERY_STRING"), query_string)?;
    ret.set_item(pyo3::intern!(py, "wsgi.url_scheme"), scheme)?;
    ret.set_item(pyo3::intern!(py, "wsgi.input"), Py::new(py, body)?)?;

    if let Some(content_type) = content_type {
        ret.set_item(
            pyo3::intern!(py, "CONTENT_TYPE"),
            content_type.to_str().unwrap_or_default(),
        )?;
    }
    if let Some(content_len) = content_len {
        ret.set_item(
            pyo3::intern!(py, "CONTENT_LENGTH"),
            content_len.to_str().unwrap_or_default(),
        )?;
    }

    ret.update(headers.into_py_dict(py).as_mapping())?;

    Ok(ret)
}
