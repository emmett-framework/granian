use hyper::header::HeaderMap;
use pyo3::{
    IntoPyObjectExt,
    prelude::*,
    pybacked::PyBackedStr,
    types::{PyBytes, PyString},
};
use tokio_tungstenite::tungstenite::Message;

use super::{
    errors::error_proto,
    types::{WebsocketInboundBytesMessage, WebsocketInboundCloseMessage, WebsocketInboundTextMessage},
};

#[inline]
pub(super) fn headers_from_py(pyheaders: Vec<(PyBackedStr, PyBackedStr)>) -> HeaderMap {
    let mut headers = HeaderMap::with_capacity(pyheaders.len() + 3);
    for (key, value) in pyheaders {
        headers.append(
            hyper::header::HeaderName::from_bytes(key.as_bytes()).unwrap(),
            hyper::header::HeaderValue::from_str(&value).unwrap(),
        );
    }
    headers.entry(hyper::header::SERVER).or_insert(crate::http::HV_SERVER);
    headers
}

#[inline]
pub(crate) fn ws_message_into_py(py: Python, message: Message) -> PyResult<Bound<PyAny>> {
    match message {
        Message::Binary(message) => {
            WebsocketInboundBytesMessage::new(PyBytes::new(py, &message).unbind()).into_bound_py_any(py)
        }
        Message::Text(message) => {
            WebsocketInboundTextMessage::new(PyString::new(py, &message).unbind()).into_bound_py_any(py)
        }
        Message::Close(_) => WebsocketInboundCloseMessage::new().into_bound_py_any(py),
        v => {
            log::warn!("Unsupported websocket message received {v:?}");
            error_proto!()
        }
    }
}
