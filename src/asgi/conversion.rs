use pyo3::{
    prelude::*,
    types::{PyBytes, PyDict},
    IntoPyObjectExt,
};
use tokio_tungstenite::tungstenite::Message;

use super::errors::error_flow;
use super::types::ASGIMessageType;
use crate::conversion::Utf8BytesToPy;

#[inline]
pub(crate) fn message_into_py(py: Python, message: ASGIMessageType) -> PyResult<Bound<PyAny>> {
    let dict = PyDict::new(py);
    match message {
        ASGIMessageType::HTTPDisconnect => {
            dict.set_item(pyo3::intern!(py, "type"), pyo3::intern!(py, "http.disconnect"))?;
        }
        ASGIMessageType::HTTPRequestBody((bytes, more)) => {
            dict.set_item(pyo3::intern!(py, "type"), pyo3::intern!(py, "http.request"))?;
            dict.set_item(pyo3::intern!(py, "body"), bytes.into_py_any(py)?)?;
            dict.set_item(pyo3::intern!(py, "more_body"), more)?;
        }
        ASGIMessageType::WSConnect => {
            dict.set_item(pyo3::intern!(py, "type"), pyo3::intern!(py, "websocket.connect"))?;
        }
        _ => unreachable!(),
    }
    Ok(dict.into_any())
}

#[inline]
pub(crate) fn ws_message_into_py(py: Python, message: Message) -> PyResult<Bound<PyAny>> {
    match message {
        Message::Binary(message) => {
            let dict = PyDict::new(py);
            dict.set_item(pyo3::intern!(py, "type"), pyo3::intern!(py, "websocket.receive"))?;
            dict.set_item(pyo3::intern!(py, "bytes"), PyBytes::new(py, &message[..]))?;
            Ok(dict.into_any())
        }
        Message::Text(message) => {
            let dict = PyDict::new(py);
            dict.set_item(pyo3::intern!(py, "type"), pyo3::intern!(py, "websocket.receive"))?;
            dict.set_item(pyo3::intern!(py, "text"), Utf8BytesToPy(message))?;
            Ok(dict.into_any())
        }
        Message::Close(frame) => {
            let close_code: u16 = match frame {
                Some(frame) => frame.code.into(),
                _ => 1005,
            };
            let dict = PyDict::new(py);
            dict.set_item(pyo3::intern!(py, "type"), pyo3::intern!(py, "websocket.disconnect"))?;
            dict.set_item(pyo3::intern!(py, "code"), close_code)?;
            Ok(dict.into_any())
        }
        v => {
            log::warn!("Unsupported websocket message received {v:?}");
            error_flow!()
        }
    }
}
