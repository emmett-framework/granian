use hyper::{
    Body,
    Request,
    Response,
    header::{HeaderName, HeaderValue, HeaderMap, SERVER}
};
use pyo3::prelude::*;
use pyo3::pyclass::PyClass;
use pyo3::types::{PyBytes, PyDict};
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use tungstenite::Message;

use crate::{
    runtime::{RuntimeRef, future_into_py},
    ws::{HyperWebsocket, UpgradeData, WebsocketTransport}
};
use super::{errors::{ASGIFlowError, UnsupportedASGIMessage}, types::ASGIMessageType};


const HDR_SERVER: HeaderValue = HeaderValue::from_static("granian");

pub(crate) trait ASGIProtocol: PyClass {
    fn _recv<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny>;
    fn _send<'p>(&mut self, py: Python<'p>, data: &'p PyDict) -> PyResult<&'p PyAny>;
}

#[pyclass(module="granian.asgi")]
pub(crate) struct HttpProtocol {
    rt: RuntimeRef,
    request: Arc<Mutex<Request<Body>>>,
    response_inited: bool,
    response_built: bool,
    response_status: i16,
    response_headers: HeaderMap,
    response_body: Vec<u8>,
    tx: Option<oneshot::Sender<Response<Body>>>
}

impl HttpProtocol {
    pub fn new(
        rt: RuntimeRef,
        request: Request<Body>,
        tx: oneshot::Sender<Response<Body>>
    ) -> Self {
        Self {
            rt: rt,
            request: Arc::new(Mutex::new(request)),
            response_inited: false,
            response_built: false,
            response_status: 0,
            response_headers: HeaderMap::new(),
            response_body: Vec::new(),
            tx: Some(tx)
        }
    }

    fn init_response(&mut self, status_code: i16, headers: HeaderMap) {
        self.response_status = status_code;
        self.response_headers = headers;
        self.response_inited = true;
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

    fn send_body(&mut self, body: &[u8], finish: bool) {
        self.response_body.extend_from_slice(body);
        if finish {
            if let Some(tx) = self.tx.take() {
                let mut res = Response::new(self.response_body.to_owned().into());
                *res.status_mut() = hyper::StatusCode::from_u16(
                    self.response_status as u16
                ).unwrap();
                *res.headers_mut() = self.response_headers.to_owned();
                let _ = tx.send(res);
            }
            self.response_built = true;
        }
    }
}

#[pymethods]
impl HttpProtocol {
    fn receive<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        self._recv(py)
    }

    fn send<'p>(&mut self, py: Python<'p>, data: &'p PyDict) -> PyResult<&'p PyAny> {
        self._send(py, data)
    }
}

#[pyclass(module="granian.asgi")]
pub(crate) struct WebsocketProtocol {
    rt: RuntimeRef,
    websocket: Arc<Mutex<WebsocketTransport>>,
    upgrade: Arc<Mutex<UpgradeData>>
}

impl WebsocketProtocol {
    pub fn new(
        rt: RuntimeRef,
        websocket: HyperWebsocket,
        upgrade: Arc<Mutex<UpgradeData>>
    ) -> Self {
        Self {
            rt: rt,
            websocket: Arc::new(Mutex::new(WebsocketTransport::new(websocket))),
            upgrade: upgrade
        }
    }

    fn adapt_message(&self, message: &PyDict) -> Message {
        let default_bytes = b"".to_vec();
        let default_string = String::new();
        match message.contains("bytes") {
            Ok(true) => {
                let data = match message.get_item("bytes") {
                    Some(item) => {
                        item.extract().unwrap_or(default_bytes)
                    },
                    _ => default_bytes
                };
                Message::Binary(data)
            },
            Ok(false) => {
                let data = match message.get_item("text") {
                    Some(item) => {
                        item.extract().unwrap_or(default_string)
                    },
                    _ => default_string
                };
                Message::Text(data)
            },
            _ => Message::Binary(b"".to_vec())
        }
    }

    fn accept<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let upgrade = self.upgrade.clone();
        let transport = self.websocket.clone();
        future_into_py(self.rt.clone(), py, async move {
            let mut res = upgrade.lock().await;
            let mut ws = transport.lock().await;
            match res.send().await {
                Ok(_) => {
                    match ws.accept().await {
                        Ok(_) => Ok(()),
                        _ => Err(ASGIFlowError.into())
                    }
                },
                _ => Err(ASGIFlowError.into())
            }
        })
    }

    fn close<'p>(&mut self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let transport = self.websocket.clone();
        future_into_py(self.rt.clone(), py, async move {
            let mut ws = transport.lock().await;
            ws.set_closed();
            Ok(())
        })
    }

    fn send_message<'p>(
        &self,
        py: Python<'p>,
        data: &'p PyDict
    ) -> PyResult<&'p PyAny> {
        let transport = self.websocket.clone();
        let message = self.adapt_message(data);
        future_into_py(self.rt.clone(), py, async move {
            let ws = transport.lock().await;
            if ws.closed {
                return Err(ASGIFlowError.into())
            }
            match ws.send(message).await {
                Ok(_) => Ok(()),
                _ => Err(ASGIFlowError.into())
            }
        })
    }
}

#[pymethods]
impl WebsocketProtocol {
    fn receive<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        self._recv(py)
    }

    fn send<'p>(&mut self, py: Python<'p>, data: &'p PyDict) -> PyResult<&'p PyAny> {
        self._send(py, data)
    }
}

impl ASGIProtocol for HttpProtocol {
    fn _recv<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let transport = self.request.clone();
        future_into_py(self.rt.clone(), py, async move {
            let mut req = transport.lock().await;
            let body = hyper::body::to_bytes(&mut *req).await.unwrap();
            Python::with_gil(|py| {
                let dict = PyDict::new(py);
                dict.set_item("type", "http.request")?;
                dict.set_item("body", &body.to_vec())?;
                dict.set_item("more_body", false)?;
                Ok(dict.to_object(py))
            })
        })
    }

    fn _send<'p>(&mut self, py: Python<'p>, data: &'p PyDict) -> PyResult<&'p PyAny> {
        match adapt_message_type(data) {
            Ok(ASGIMessageType::HTTPStart) => {
                match self.response_inited {
                    false => {
                        self.init_response(
                            self.adapt_status_code(data).unwrap(),
                            self.adapt_headers(data)
                        );
                        empty_future(self.rt.clone(), py)
                    },
                    _ => Err(ASGIFlowError.into())
                }
            },
            Ok(ASGIMessageType::HTTPBody) => {
                match (self.response_inited, self.response_built) {
                    (true, false) => {
                        let body_data = self.adapt_body(data);
                        self.send_body(&body_data.0[..], !body_data.1);
                        empty_future(self.rt.clone(), py)
                    },
                    _ => Err(ASGIFlowError.into())
                }
            },
            Err(err) => Err(err.into()),
            _ => Err(UnsupportedASGIMessage.into())
        }
    }
}

impl ASGIProtocol for WebsocketProtocol {
    fn _recv<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let transport = self.websocket.clone();
        future_into_py(self.rt.clone(), py, async move {
            let ws = transport.lock().await;
            if ws.closed {
                return Err(ASGIFlowError.into())
            }
            match ws.receive().await {
                Ok(message) => {
                    match message {
                        Message::Binary(message) => {
                            Python::with_gil(|py| {
                                let dict = PyDict::new(py);
                                dict.set_item("type", "websocket.receive")?;
                                dict.set_item("bytes", PyBytes::new(py, &message[..]))?;
                                Ok(dict.to_object(py))
                            })
                        },
                        Message::Text(message) => {
                            Python::with_gil(|py| {
                                let dict = PyDict::new(py);
                                dict.set_item("type", "websocket.receive")?;
                                dict.set_item("text", message)?;
                                Ok(dict.to_object(py))
                            })
                        },
                        Message::Close(_) => {
                            Python::with_gil(|py| {
                                let dict = PyDict::new(py);
                                dict.set_item("type", "websocket.disconnect")?;
                                Ok(dict.to_object(py))
                            })
                        },
                        _ => Err(ASGIFlowError.into())
                    }
                },
                _ => Err(ASGIFlowError.into())
            }
        })
    }

    fn _send<'p>(&mut self, py: Python<'p>, data: &'p PyDict) -> PyResult<&'p PyAny> {
        match adapt_message_type(data) {
            Ok(ASGIMessageType::WSAccept) => {
                self.accept(py)
            },
            Ok(ASGIMessageType::WSClose) => {
                self.close(py)
            },
            Ok(ASGIMessageType::WSMessage) => {
                self.send_message(py, data)
            },
            Err(err) => Err(err.into()),
            _ => Err(UnsupportedASGIMessage.into())
        }
    }
}

fn adapt_message_type(
    message: &PyDict
) -> Result<ASGIMessageType, UnsupportedASGIMessage> {
    match message.get_item("type") {
        Some(item) => {
            let message_type: &str = item.extract()?;
            match message_type {
                "http.response.start" => Ok(ASGIMessageType::HTTPStart),
                "http.response.body" => Ok(ASGIMessageType::HTTPBody),
                "websocket.accept" => Ok(ASGIMessageType::WSAccept),
                "websocket.close" => Ok(ASGIMessageType::WSClose),
                "websocket.send" => Ok(ASGIMessageType::WSMessage),
                _ => Err(UnsupportedASGIMessage)
            }
        },
        _ => Err(UnsupportedASGIMessage)
    }
}

fn empty_future<'p>(rt: RuntimeRef, py: Python<'p>) -> PyResult<&'p PyAny> {
    future_into_py(rt, py, async move {
        Ok(())
    })
}
