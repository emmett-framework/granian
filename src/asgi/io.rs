use bytes::Buf;
use hyper::{
    Body,
    Response,
    header::{HeaderName, HeaderValue, HeaderMap, SERVER}
};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use tokio::sync::oneshot;

use super::errors::{ASGIFlowError, UnsupportedASGIMessage};
use super::types::ASGIMessageType;

const HDR_SERVER: HeaderValue = HeaderValue::from_static("granian");

#[pyclass(module="granian.asgi")]
pub(crate) struct Sender {
    inited: bool,
    consumed: bool,
    status: i16,
    headers: HeaderMap,
    body: Vec<u8>,
    tx: Option<oneshot::Sender<Response<Body>>>
}

impl Sender {
    pub fn new(tx: Option<oneshot::Sender<Response<Body>>>) -> Self {
        Self {
            inited: false,
            consumed: false,
            status: 0,
            headers: HeaderMap::new(),
            body: Vec::new(),
            tx: tx
        }
    }

    fn adapt_message_type(
        &self,
        message: &PyDict
    ) -> Result<ASGIMessageType, UnsupportedASGIMessage> {
        match message.get_item("type") {
            Some(item) => {
                let message_type: &str = item.extract()?;
                match message_type {
                    "http.response.start" => Ok(ASGIMessageType::Start),
                    "http.response.body" => Ok(ASGIMessageType::Body),
                    _ => Err(UnsupportedASGIMessage)
                }
            },
            _ => Err(UnsupportedASGIMessage)
        }
    }

    fn adapt_status_code(
        &self,
        message: &PyDict
    ) -> Result<i16, UnsupportedASGIMessage> {
        match message.get_item("status") {
            Some(item) => {
                Ok(item.extract()?)
            },
            _ => Err(UnsupportedASGIMessage)
        }
    }

    fn adapt_headers(&self, message: &PyDict) -> HeaderMap {
        let mut ret = HeaderMap::new();
        ret.insert(SERVER, HDR_SERVER);
        match message.get_item("headers") {
            Some(item) => {
                let accum: Vec<Vec<&[u8]>> = item.extract().unwrap_or(Vec::new());
                for tup in accum.iter() {
                    match (
                        HeaderName::from_bytes(tup[0]),
                        HeaderValue::from_bytes(tup[1])
                     ) {
                        (Ok(key), Ok(val)) => { ret.insert(key, val); },
                        _ => {}
                    }
                };
                ret
            },
            _ => ret
        }
    }

    fn adapt_body(&self, message: &PyDict) -> (Vec<u8>, bool) {
        let default_body = b"".to_vec();
        let default_more = false;
        let body = match message.get_item("body") {
            Some(item) => {
                item.extract().unwrap_or(default_body)
            },
            _ => default_body
        };
        let more = match message.get_item("more_body") {
            Some(item) => {
                item.extract().unwrap_or(default_more)
            },
            _ => default_more
        };
        (body, more)
    }

    fn init_response(&mut self, status_code: i16, headers: HeaderMap) {
        self.status = status_code;
        self.headers = headers;
        self.inited = true;
    }

    fn send_body(&mut self, body: &[u8], finish: bool) {
        self.body.extend_from_slice(body);
        if finish {
            if let Some(tx) = self.tx.take() {
                let mut res = Response::new(self.body.to_owned().into());
                *res.status_mut() = hyper::StatusCode::from_u16(
                    self.status as u16
                ).unwrap();
                *res.headers_mut() = self.headers.to_owned();
                let _ = tx.send(res);
            }
            self.consumed = true
        }
    }
}

#[pymethods]
impl Sender {
    fn __call__<'p>(&mut self, data: &PyDict) -> PyResult<()> {
        match self.adapt_message_type(data) {
            Ok(ASGIMessageType::Start) => {
                match self.inited {
                    false => {
                        self.init_response(
                            self.adapt_status_code(data).unwrap(),
                            self.adapt_headers(data)
                        );
                        Ok(())
                    },
                    _ => Err(ASGIFlowError.into())
                }
            },
            Ok(ASGIMessageType::Body) => {
                match (self.inited, self.consumed) {
                    (true, false) => {
                        let body_data = self.adapt_body(data);
                        self.send_body(&body_data.0[..], !body_data.1);
                        Ok(())
                    },
                    _ => Err(ASGIFlowError.into())
                }
            },
            Err(err) => Err(err.into())
        }
    }
}
