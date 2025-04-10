use futures::StreamExt;
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

// NOTE: for unknown reasons, under some circumstances (`threading` module usage in app?)
//       this gets shared across threads. So it can't be `unsendable` (yet?).
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
        self.tx.lock().map_or(None, |mut v| v.take())
    }
}

macro_rules! headers_from_py {
    ($headers:expr) => {{
        let mut headers = HeaderMap::with_capacity($headers.len() + 3);
        for (key, value) in $headers {
            headers.append(
                HeaderName::from_bytes(key.as_bytes()).unwrap(),
                HeaderValue::from_str(&value).unwrap(),
            );
        }
        headers.entry(HK_SERVER).or_insert(HV_SERVER);
        headers
    }};
}

#[pymethods]
impl WSGIProtocol {
    fn response_bytes(&self, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, body: Cow<[u8]>) {
        if let Some(tx) = self.tx.lock().map_or(None, |mut v| v.take()) {
            let data: Box<[u8]> = body.into();
            let txbody = http_body_util::Full::new(body::Bytes::from(data))
                .map_err(|e| match e {})
                .boxed();
            let _ = tx.send((status, headers_from_py!(headers), txbody));
        }
    }

    fn response_iter(&self, py: Python, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, body: Bound<PyAny>) {
        if let Some(tx) = self.tx.lock().map_or(None, |mut v| v.take()) {
            let (body_tx, body_rx) = mpsc::unbounded_channel::<body::Bytes>();

            let body_stream = http_body_util::StreamBody::new(
                tokio_stream::wrappers::UnboundedReceiverStream::new(body_rx)
                    .map(body::Frame::data)
                    .map(Result::Ok),
            );
            let txbody = BodyExt::boxed(body_stream);
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
                            log_application_callable_exception(py, &err);
                        }
                        let _ = body.call_method0(pyo3::intern!(py, "close"));
                        closed = true;
                        None
                    }
                } {
                    if body_tx.send(frame).is_ok() {
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
