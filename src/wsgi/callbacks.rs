use hyper::Body;
use pyo3::prelude::*;
use tokio::task::JoinHandle;

use crate::callbacks::CallbackWrapper;
use super::{
    errors::error_proto,
    types::{WSGIScope as Scope, WSGIResponseBodyIter}
};

const WSGI_LIST_RESPONSE_BODY: i32 = 0;
const WSGI_ITER_RESPONSE_BODY: i32 = 1;


pub(crate) async fn call_rtb_http(
    cb: CallbackWrapper,
    scope: Scope
) -> PyResult<(i32, Vec<(String, String)>, Body)> {
    let callback = cb.callback.clone();

    Python::with_gil(|py| {
        let (status, headers, body_type, pybody) = callback.call1(py, (scope,))?
            .extract::<(i32, Vec<(String, String)>, i32, PyObject)>(py)?;
        let body = match body_type {
            WSGI_LIST_RESPONSE_BODY => Body::from(pybody.extract::<Vec<u8>>(py)?),
            WSGI_ITER_RESPONSE_BODY => Body::wrap_stream(WSGIResponseBodyIter::new(pybody)),
            _ => Body::empty()
        };
        Ok((status, headers, body))
    })
}

pub(crate) async fn call_rtt_http(
    cb: CallbackWrapper,
    scope: Scope
) -> PyResult<(i32, Vec<(String, String)>, Body)> {
    let callback = cb.callback.clone();

    let fut: JoinHandle<PyResult<(i32, Vec<(String, String)>, Body)>> = tokio::task::spawn_blocking(move || {
        Python::with_gil(|py| {
            let (status, headers, body_type, pybody) = callback.call1(py, (scope,))?
                .extract::<(i32, Vec<(String, String)>, i32, PyObject)>(py)?;
            let body = match body_type {
                WSGI_LIST_RESPONSE_BODY => Body::from(pybody.extract::<Vec<u8>>(py)?),
                WSGI_ITER_RESPONSE_BODY => Body::wrap_stream(WSGIResponseBodyIter::new(pybody)),
                _ => Body::empty()
            };
            Ok((status, headers, body))
        })
    });

    match fut.await {
        Ok(res) => res,
        _ => {
            log::error!("WSGI protocol failure");
            error_proto!()
        }
    }
}
