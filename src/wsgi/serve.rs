use pyo3::prelude::*;

use super::http::{handle_rtb, handle_rtt};
use crate::workers::{serve_rth, serve_rth_ssl, serve_wth, serve_wth_ssl, WorkerConfig};

#[pyclass(module = "granian._granian")]
pub struct WSGIWorker {
    config: WorkerConfig,
}

impl WSGIWorker {
    serve_rth!(_serve_rth, handle_rtb);
    serve_wth!(_serve_wth, handle_rtt);
    serve_rth_ssl!(_serve_rth_ssl, handle_rtb);
    serve_wth_ssl!(_serve_wth_ssl, handle_rtt);
}

#[pymethods]
impl WSGIWorker {
    #[new]
    #[pyo3(
        signature = (
            worker_id,
            socket_fd,
            threads=1,
            pthreads=1,
            http_mode="1",
            http1_buffer_max=65535,
            ssl_enabled=false,
            ssl_cert=None,
            ssl_key=None
        )
    )]
    fn new(
        worker_id: i32,
        socket_fd: i32,
        threads: usize,
        pthreads: usize,
        http_mode: &str,
        http1_buffer_max: usize,
        ssl_enabled: bool,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
    ) -> PyResult<Self> {
        Ok(Self {
            config: WorkerConfig::new(
                worker_id,
                socket_fd,
                threads,
                pthreads,
                http_mode,
                http1_buffer_max,
                false,
                true,
                ssl_enabled,
                ssl_cert,
                ssl_key,
            ),
        })
    }

    fn serve_rth(&self, callback: PyObject, event_loop: &PyAny, context: &PyAny, signal_rx: PyObject) {
        match self.config.ssl_enabled {
            false => self._serve_rth(callback, event_loop, context, signal_rx),
            true => self._serve_rth_ssl(callback, event_loop, context, signal_rx),
        }
    }

    fn serve_wth(&self, callback: PyObject, event_loop: &PyAny, context: &PyAny, signal_rx: PyObject) {
        match self.config.ssl_enabled {
            false => self._serve_wth(callback, event_loop, context, signal_rx),
            true => self._serve_wth_ssl(callback, event_loop, context, signal_rx),
        }
    }
}
