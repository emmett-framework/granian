use pyo3::prelude::*;

use super::http::{handle, handle_ws};

use crate::callbacks::CallbackScheduler;
use crate::conversion::{worker_http1_config_from_py, worker_http2_config_from_py};
use crate::tcp::ListenerSpec;
use crate::workers::{
    serve_fut, serve_fut_ssl, serve_mtr, serve_mtr_ssl, serve_str, serve_str_ssl, WorkerConfig, WorkerSignal,
};

#[pyclass(frozen, module = "granian._granian")]
pub struct ASGIWorker {
    config: WorkerConfig,
}

impl ASGIWorker {
    serve_mtr!(_serve_mtr, handle);
    serve_mtr!(_serve_mtr_ws, handle_ws);
    serve_str!(_serve_str, handle);
    serve_str!(_serve_str_ws, handle_ws);
    serve_fut!(_serve_fut, handle);
    serve_fut!(_serve_fut_ws, handle_ws);
    serve_mtr_ssl!(_serve_mtr_ssl, handle);
    serve_mtr_ssl!(_serve_mtr_ssl_ws, handle_ws);
    serve_str_ssl!(_serve_str_ssl, handle);
    serve_str_ssl!(_serve_str_ssl_ws, handle_ws);
    serve_fut_ssl!(_serve_fut_ssl, handle);
    serve_fut_ssl!(_serve_fut_ssl_ws, handle_ws);
}

#[pymethods]
impl ASGIWorker {
    #[new]
    #[pyo3(
        signature = (
            worker_id,
            sock,
            threads=1,
            blocking_threads=512,
            py_threads=1,
            py_threads_idle_timeout=30,
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
        sock: (Py<ListenerSpec>, Option<i32>),
        threads: usize,
        blocking_threads: usize,
        py_threads: usize,
        py_threads_idle_timeout: u64,
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
                sock,
                threads,
                blocking_threads,
                py_threads,
                py_threads_idle_timeout,
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

    fn serve_mtr(
        &self,
        py: Python,
        callback: Py<CallbackScheduler>,
        event_loop: &Bound<PyAny>,
        signal: Py<WorkerSignal>,
    ) {
        match (self.config.websockets_enabled, self.config.ssl_enabled) {
            (false, false) => self._serve_mtr(py, callback, event_loop, signal),
            (true, false) => self._serve_mtr_ws(py, callback, event_loop, signal),
            (false, true) => self._serve_mtr_ssl(py, callback, event_loop, signal),
            (true, true) => self._serve_mtr_ssl_ws(py, callback, event_loop, signal),
        }
    }

    fn serve_str(&self, callback: Py<CallbackScheduler>, event_loop: &Bound<PyAny>, signal: Py<WorkerSignal>) {
        match (self.config.websockets_enabled, self.config.ssl_enabled) {
            (false, false) => self._serve_str(callback, event_loop, signal),
            (true, false) => self._serve_str_ws(callback, event_loop, signal),
            (false, true) => self._serve_str_ssl(callback, event_loop, signal),
            (true, true) => self._serve_str_ssl_ws(callback, event_loop, signal),
        }
    }

    fn serve_async<'p>(
        &self,
        callback: Py<CallbackScheduler>,
        event_loop: &Bound<'p, PyAny>,
        signal: Py<WorkerSignal>,
    ) -> Bound<'p, PyAny> {
        match (self.config.websockets_enabled, self.config.ssl_enabled) {
            (false, false) => self._serve_fut(callback, event_loop, signal),
            (true, false) => self._serve_fut_ws(callback, event_loop, signal),
            (false, true) => self._serve_fut_ssl(callback, event_loop, signal),
            (true, true) => self._serve_fut_ssl_ws(callback, event_loop, signal),
        }
    }
}
