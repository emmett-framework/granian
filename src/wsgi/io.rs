use futures::StreamExt;
use http_body_util::BodyExt;
use hyper::{body, header::HeaderMap};
use pyo3::{prelude::*, pybacked::PyBackedStr};
use std::{borrow::Cow, sync::Mutex, time::Instant};
use tokio::sync::{mpsc, oneshot};

use crate::{
    conversion::headers_from_py, http::HTTPResponseBody, runtime::RequestRt, utils::log_application_callable_exception,
};

// NOTE: for unknown reasons, under some circumstances (`threading` module usage in app?)
//       this gets shared across threads. So it can't be `unsendable` (yet?).
#[pyclass(frozen)]
pub(super) struct WSGIProtocol {
    tx: Mutex<Option<oneshot::Sender<(u16, HeaderMap, HTTPResponseBody)>>>,
    rt: RequestRt,
    acct: Mutex<Instant>,
}

impl WSGIProtocol {
    pub fn new(rt: RequestRt, tx: oneshot::Sender<(u16, HeaderMap, HTTPResponseBody)>) -> Self {
        Self {
            tx: Mutex::new(Some(tx)),
            rt,
            acct: Mutex::new(Instant::now()),
        }
    }

    pub fn tx(&self) -> Option<oneshot::Sender<(u16, HeaderMap, HTTPResponseBody)>> {
        self.tx.lock().map_or(None, |mut v| v.take())
    }

    /// Restart the occupancy clock; called when the request task actually starts.
    pub fn start(&self) {
        *self.acct.lock().unwrap() = Instant::now();
    }

    /// Charge the lane occupancy accumulated since the previous debit to the request's flow.
    pub fn debit(&self) {
        let cost = {
            let mut last = self.acct.lock().unwrap();
            let now = Instant::now();
            let cost = now.duration_since(*last).as_nanos() as u64;
            *last = now;
            cost
        };
        if cost > 0
            && let Some(flow) = self.rt.flow
        {
            self.rt.rt.debit_flow(&flow, cost);
        }
    }

    /// Final flush and flow release, at the end of the request task.
    pub fn finalize(&self) {
        self.debit();
        self.rt.release();
    }
}

#[pymethods]
impl WSGIProtocol {
    fn response_bytes(&self, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, body: Cow<[u8]>) {
        self.debit();
        if let Some(tx) = self.tx.lock().map_or(None, |mut v| v.take()) {
            let data: Box<[u8]> = body.into();
            let txbody = http_body_util::Full::new(body::Bytes::from(data))
                .map_err(|e| match e {})
                .boxed();
            let _ = tx.send((status, headers_from_py(headers), txbody));
        }
    }

    fn response_iter(&self, py: Python, status: u16, headers: Vec<(PyBackedStr, PyBackedStr)>, body: Bound<PyAny>) {
        if let Some(tx) = self.tx.lock().map_or(None, |mut v| v.take()) {
            //: chan capacity 2 (the actual number we need for pipelining) * 2 to have some "margin"
            let (body_tx, body_rx) = mpsc::channel::<body::Bytes>(4);

            let body_stream = http_body_util::StreamBody::new(
                tokio_stream::wrappers::ReceiverStream::new(body_rx)
                    .map(body::Frame::data)
                    .map(Result::Ok),
            );
            let txbody = BodyExt::boxed(body_stream);
            let _ = tx.send((status, headers_from_py(headers), txbody));

            let mut closed = false;
            loop {
                self.debit();
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
                    match body_tx.try_send(frame) {
                        Ok(()) => continue,
                        Err(mpsc::error::TrySendError::Full(frame)) => {
                            if py.detach(|| body_tx.blocking_send(frame)).is_ok() {
                                continue;
                            }
                        }
                        _ => {}
                    }
                }

                if !closed {
                    let _ = body.call_method0(pyo3::intern!(py, "close"));
                }
                break;
            }
            self.debit();
        }
    }
}
