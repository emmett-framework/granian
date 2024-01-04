use futures::TryStreamExt;
use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use hyper::body::Bytes;
use pyo3::prelude::*;
use std::borrow::Cow;
use tokio::task::JoinHandle;

use super::types::{WSGIResponseBodyIter, WSGIScope as Scope};
use crate::callbacks::CallbackWrapper;
use crate::http::empty_body;

const WSGI_BYTES_RESPONSE_BODY: i32 = 0;
const WSGI_ITER_RESPONSE_BODY: i32 = 1;

#[inline(always)]
fn run_callback(
    callback: PyObject,
    scope: Scope,
) -> PyResult<(i32, Vec<(String, String)>, BoxBody<Bytes, anyhow::Error>)> {
    Python::with_gil(|py| {
        let (status, headers, body_type, pybody) =
            callback
                .call1(py, (scope,))?
                .extract::<(i32, Vec<(String, String)>, i32, PyObject)>(py)?;
        let body = match body_type {
            WSGI_BYTES_RESPONSE_BODY => {
                let data: Box<[u8]> = pybody.extract::<Cow<[u8]>>(py)?.into();
                http_body_util::Full::new(Bytes::from(data))
                    .map_err(|e| match e {})
                    .boxed()
                // Body::from(Bytes::from(data))
            }
            WSGI_ITER_RESPONSE_BODY => {
                let body = http_body_util::StreamBody::new(
                    WSGIResponseBodyIter::new(pybody).map_ok(|v| hyper::body::Frame::data(Bytes::from(v))),
                );
                BodyExt::boxed(BodyExt::map_err(body, |e| match e {}))
            }
            _ => empty_body(),
        };
        Ok((status, headers, body))
    })
}

#[inline(always)]
pub(crate) fn call_rtb_http(
    cb: CallbackWrapper,
    scope: Scope,
) -> PyResult<(i32, Vec<(String, String)>, BoxBody<Bytes, anyhow::Error>)> {
    run_callback(cb.callback, scope)
}

#[inline(always)]
pub(crate) fn call_rtt_http(
    cb: CallbackWrapper,
    scope: Scope,
) -> JoinHandle<PyResult<(i32, Vec<(String, String)>, BoxBody<Bytes, anyhow::Error>)>> {
    tokio::task::spawn_blocking(move || run_callback(cb.callback, scope))
}
