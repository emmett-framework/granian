use pyo3::prelude::*;

use crate::workers::{HTTP1Config, HTTP2Config};

pub(crate) struct BytesToPy(pub hyper::body::Bytes);

impl IntoPy<PyObject> for BytesToPy {
    #[inline]
    fn into_py(self, py: Python) -> PyObject {
        self.0.as_ref().into_py(py)
    }
}

impl ToPyObject for BytesToPy {
    #[inline]
    fn to_object(&self, py: Python<'_>) -> PyObject {
        self.0.as_ref().into_py(py)
    }
}

pub(crate) fn worker_http1_config_from_py(py: Python, cfg: Option<PyObject>) -> PyResult<HTTP1Config> {
    let ret = match cfg {
        Some(cfg) => HTTP1Config {
            keep_alive: cfg.getattr(py, "keep_alive")?.extract(py)?,
            max_buffer_size: cfg.getattr(py, "max_buffer_size")?.extract(py)?,
            pipeline_flush: cfg.getattr(py, "pipeline_flush")?.extract(py)?,
        },
        None => HTTP1Config {
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
            keep_alive_interval: match cfg.getattr(py, "keep_alive_interval")?.extract(py) {
                Ok(v) => Some(core::time::Duration::from_secs(v)),
                _ => None,
            },
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
