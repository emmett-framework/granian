use pyo3::{prelude::*, IntoPyObjectExt};

use crate::workers::{HTTP1Config, HTTP2Config};

pub(crate) struct BytesToPy(pub hyper::body::Bytes);
pub(crate) struct Utf8BytesToPy(pub tokio_tungstenite::tungstenite::Utf8Bytes);

impl<'p> IntoPyObject<'p> for BytesToPy {
    type Target = PyAny;
    type Output = Bound<'p, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'p>) -> Result<Self::Output, Self::Error> {
        self.0.as_ref().into_bound_py_any(py)
    }
}

impl<'p> IntoPyObject<'p> for Utf8BytesToPy {
    type Target = PyAny;
    type Output = Bound<'p, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'p>) -> Result<Self::Output, Self::Error> {
        self.0.as_str().into_bound_py_any(py)
    }
}

pub(crate) enum FutureResultToPy {
    None,
    Err(PyResult<()>),
    Bytes(hyper::body::Bytes),
    ASGIMessage(crate::asgi::types::ASGIMessageType),
    ASGIWSMessage(tokio_tungstenite::tungstenite::Message),
    RSGIWSAccept(crate::rsgi::io::RSGIWebsocketTransport),
    RSGIWSMessage(tokio_tungstenite::tungstenite::Message),
}

impl<'p> IntoPyObject<'p> for FutureResultToPy {
    type Target = PyAny;
    type Output = Bound<'p, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'p>) -> Result<Self::Output, Self::Error> {
        match self {
            Self::None => Ok(py.None().into_bound(py)),
            Self::Err(res) => Err(res.err().unwrap()),
            Self::Bytes(inner) => inner.into_pyobject(py),
            Self::ASGIMessage(message) => crate::asgi::conversion::message_into_py(py, message),
            Self::ASGIWSMessage(message) => crate::asgi::conversion::ws_message_into_py(py, message),
            Self::RSGIWSAccept(obj) => obj.into_bound_py_any(py),
            Self::RSGIWSMessage(message) => crate::rsgi::conversion::ws_message_into_py(py, message),
        }
    }
}

pub(crate) fn worker_http1_config_from_py(py: Python, cfg: Option<PyObject>) -> PyResult<HTTP1Config> {
    let ret = match cfg {
        Some(cfg) => HTTP1Config {
            header_read_timeout: cfg
                .getattr(py, "header_read_timeout")?
                .extract(py)
                .map(core::time::Duration::from_millis)?,
            keep_alive: cfg.getattr(py, "keep_alive")?.extract(py)?,
            max_buffer_size: cfg.getattr(py, "max_buffer_size")?.extract(py)?,
            pipeline_flush: cfg.getattr(py, "pipeline_flush")?.extract(py)?,
        },
        None => HTTP1Config {
            header_read_timeout: core::time::Duration::from_secs(30),
            keep_alive: true,
            max_buffer_size: 8192 + 4096 * 100,
            pipeline_flush: false,
        },
    };
    Ok(ret)
}

pub(crate) fn worker_http2_config_from_py(py: Python, cfg: Option<PyObject>) -> PyResult<HTTP2Config> {
    let ret = match cfg {
        Some(cfg) => HTTP2Config {
            adaptive_window: cfg.getattr(py, "adaptive_window")?.extract(py)?,
            initial_connection_window_size: cfg.getattr(py, "initial_connection_window_size")?.extract(py)?,
            initial_stream_window_size: cfg.getattr(py, "initial_stream_window_size")?.extract(py)?,
            keep_alive_interval: cfg
                .getattr(py, "keep_alive_interval")?
                .extract(py)
                .ok()
                .map(core::time::Duration::from_millis),
            keep_alive_timeout: core::time::Duration::from_secs(cfg.getattr(py, "keep_alive_timeout")?.extract(py)?),
            max_concurrent_streams: cfg.getattr(py, "max_concurrent_streams")?.extract(py)?,
            max_frame_size: cfg.getattr(py, "max_frame_size")?.extract(py)?,
            max_headers_size: cfg.getattr(py, "max_headers_size")?.extract(py)?,
            max_send_buffer_size: cfg.getattr(py, "max_send_buffer_size")?.extract(py)?,
        },
        None => HTTP2Config {
            adaptive_window: false,
            initial_connection_window_size: 1024 * 1024,
            initial_stream_window_size: 1024 * 1024,
            keep_alive_interval: None,
            keep_alive_timeout: core::time::Duration::from_secs(20),
            max_concurrent_streams: 200,
            max_frame_size: 1024 * 16,
            max_headers_size: 16 * 1024 * 1024,
            max_send_buffer_size: 1024 * 400,
        },
    };
    Ok(ret)
}
