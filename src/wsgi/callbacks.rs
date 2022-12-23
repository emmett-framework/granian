use pyo3::prelude::*;
use tokio::task::JoinHandle;

use crate::callbacks::CallbackWrapper;
use super::{
    errors::error_proto,
    types::WSGIScope as Scope
};


pub(crate) async fn call_rtb_http(
    cb: CallbackWrapper,
    scope: Scope
) -> PyResult<(i32, Vec<(String, String)>, Vec<u8>)> {
    let callback = cb.callback.clone();

    let fut = Python::with_gil(|py| {
        callback.call1(py, (scope,))?
            .extract::<(i32, Vec<(String, String)>, Vec<u8>)>(py)
    });

    match fut {
        Ok(res) => Ok(res),
        _ => error_proto!()
    }
}

pub(crate) async fn call_rtt_http(
    cb: CallbackWrapper,
    scope: Scope
) -> PyResult<(i32, Vec<(String, String)>, Vec<u8>)> {
    let callback = cb.callback.clone();

    let fut: JoinHandle<PyResult<(i32, Vec<(String, String)>, Vec<u8>)>> = tokio::task::spawn_blocking(move || {
        Python::with_gil(|py| {
            let res = callback.call1(py, (scope,))?.extract(py)?;
            Ok(res)
        })
    });

    match fut.await {
        Ok(res) => {
            match res {
                Ok(res) => Ok(res),
                _ => error_proto!()
            }
        },
        _ => {
            log::error!("WSGI protocol failure");
            error_proto!()
        }
    }
}
