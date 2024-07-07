use futures::TryStreamExt;
use http_body_util::BodyExt;
use hyper::{
    body,
    header::{HeaderMap, HeaderName, HeaderValue, SERVER as HK_SERVER},
};
use pyo3::{prelude::*, pybacked::PyBackedStr};
use std::{borrow::Cow, sync::Mutex};
use tokio::sync::{mpsc, oneshot};

use crate::{
    http::{HTTPResponseBody, HV_SERVER},
    utils::log_application_callable_exception,
};

#[pyclass(frozen)]
pub(super) struct WSGIProtocol {
    tx: Mutex<Option<oneshot::Sender<(u16, HeaderMap, HTTPResponseBody)>>>,
}

impl WSGIProtocol {
    pub fn new(tx: oneshot::Sender<(u16, HeaderMap, HTTPResponseBody)>) -> Self {
        Self {
            tx: Mutex::new(Some(tx)),
        }
    }

    pub fn tx(&self) -> Option<oneshot::Sender<(u16, HeaderMap, HTTPResponseBody)>> {
        self.tx.lock().unwrap().take()
    }
}

macro_rules! headers_from_py {
    ($headers:expr) => {{
        let mut headers = HeaderMap::with_capacity($headers.len() + 3);
        headers.insert(HK_SERVER, HV_SERVER);
        for (key, value) in $headers {
            headers.append(
                HeaderName::from_bytes(key.as_bytes()).unwrap(),
                HeaderValue::from_str(&value).unwrap(),
            );
        }
        headers
    }};
}

#[pymethods]
impl WSGIProtocol {
    fn response_bytes(&self, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, body: Cow<[u8]>) {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            let data: Box<[u8]> = body.into();
            let txbody = http_body_util::Full::new(body::Bytes::from(data))
                .map_err(|e| match e {})
                .boxed();
            let _ = tx.send((status, headers_from_py!(headers), txbody));
        }
    }

    fn response_iter(&self, py: Python, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, body: Bound<PyAny>) {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            let (body_tx, body_rx) = mpsc::channel::<Result<body::Bytes, anyhow::Error>>(1);

            let body_stream = http_body_util::StreamBody::new(
                tokio_stream::wrappers::ReceiverStream::new(body_rx).map_ok(body::Frame::data),
            );
            let txbody = BodyExt::boxed(BodyExt::map_err(body_stream, std::convert::Into::into));
            let _ = tx.send((status, headers_from_py!(headers), txbody));

            let mut closed = false;
            loop {
                if let Some(frame) = match body.call_method0(pyo3::intern!(py, "__next__")) {
                    Ok(chunk_obj) => match chunk_obj.extract::<Cow<[u8]>>() {
                        Ok(chunk) => {
                            let chunk: Box<[u8]> = chunk.into();
                            Some(body::Bytes::from(chunk))
                        }
                        _ => {
                            let _ = body.call_method0(pyo3::intern!(py, "close"));
                            closed = true;
                            None
                        }
                    },
                    Err(err) => {
                        if !err.is_instance_of::<pyo3::exceptions::PyStopIteration>(py) {
                            log_application_callable_exception(&err);
                        }
                        let _ = body.call_method0(pyo3::intern!(py, "close"));
                        closed = true;
                        None
                    }
                } {
                    if py.allow_threads(|| body_tx.blocking_send(Ok(frame))).is_ok() {
                        continue;
                    }
                }

                if !closed {
                    let _ = body.call_method0(pyo3::intern!(py, "close"));
                }
                break;
            }
        }
    }
}
