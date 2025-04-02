use pyo3::{
    prelude::*,
    types::{PyBytes, PyString},
    IntoPyObjectExt,
};
use tokio_tungstenite::tungstenite::Message;

use super::errors::error_proto;
use super::types::{WebsocketInboundBytesMessage, WebsocketInboundCloseMessage, WebsocketInboundTextMessage};

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
