use pyo3::prelude::*;

use super::http::{handle, handle_ws};

use crate::callbacks::CallbackScheduler;
use crate::conversion::{worker_http1_config_from_py, worker_http2_config_from_py};
use crate::workers::{serve_rth, serve_rth_ssl, serve_wth, serve_wth_ssl, WorkerConfig, WorkerSignal, WorkerSignals};

#[pyclass(frozen, module = "granian._granian")]
pub struct RSGIWorker {
    config: WorkerConfig,
}

impl RSGIWorker {
    serve_rth!(_serve_rth, handle);
    serve_rth!(_serve_rth_ws, handle_ws);
    serve_wth!(_serve_wth, handle);
    serve_wth!(_serve_wth_ws, handle_ws);
    serve_rth_ssl!(_serve_rth_ssl, handle);
    serve_rth_ssl!(_serve_rth_ssl_ws, handle_ws);
    serve_wth_ssl!(_serve_wth_ssl, handle);
    serve_wth_ssl!(_serve_wth_ssl_ws, handle_ws);
}

#[pymethods]
impl RSGIWorker {
    #[new]
    #[pyo3(
        signature = (
            worker_id,
            socket_fd,
            threads=1,
            blocking_threads=512,
            backpressure=256,
            http_mode="1",
            http1_opts=None,
            http2_opts=None,
            websockets_enabled=false,
            ssl_enabled=false,
            ssl_cert=None,
            ssl_key=None,
            ssl_key_password=None
        )
    )]
    fn new(
        py: Python,
        worker_id: i32,
        socket_fd: i32,
        threads: usize,
        blocking_threads: usize,
        backpressure: usize,
        http_mode: &str,
        http1_opts: Option<PyObject>,
        http2_opts: Option<PyObject>,
        websockets_enabled: bool,
        ssl_enabled: bool,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
        ssl_key_password: Option<&str>,
    ) -> PyResult<Self> {
        Ok(Self {
            config: WorkerConfig::new(
                worker_id,
                socket_fd,
                threads,
                blocking_threads,
                backpressure,
                http_mode,
                worker_http1_config_from_py(py, http1_opts)?,
                worker_http2_config_from_py(py, http2_opts)?,
                websockets_enabled,
                ssl_enabled,
                ssl_cert,
                ssl_key,
                ssl_key_password,
            ),
        })
    }

    fn serve_rth(&self, callback: Py<CallbackScheduler>, event_loop: &Bound<PyAny>, signal: Py<WorkerSignal>) {
        match (self.config.websockets_enabled, self.config.ssl_enabled) {
            (false, false) => self._serve_rth(callback, event_loop, WorkerSignals::Tokio(signal)),
            (true, false) => self._serve_rth_ws(callback, event_loop, WorkerSignals::Tokio(signal)),
            (false, true) => self._serve_rth_ssl(callback, event_loop, WorkerSignals::Tokio(signal)),
            (true, true) => self._serve_rth_ssl_ws(callback, event_loop, WorkerSignals::Tokio(signal)),
        }
    }

    fn serve_wth(&self, callback: Py<CallbackScheduler>, event_loop: &Bound<PyAny>, signal: Py<WorkerSignal>) {
        match (self.config.websockets_enabled, self.config.ssl_enabled) {
            (false, false) => self._serve_wth(callback, event_loop, WorkerSignals::Tokio(signal)),
            (true, false) => self._serve_wth_ws(callback, event_loop, WorkerSignals::Tokio(signal)),
            (false, true) => self._serve_wth_ssl(callback, event_loop, WorkerSignals::Tokio(signal)),
            (true, true) => self._serve_wth_ssl_ws(callback, event_loop, WorkerSignals::Tokio(signal)),
        }
    }
}
