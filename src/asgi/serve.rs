use pyo3::prelude::*;

use super::http::{handle, handle_pyw, handle_ws, handle_ws_pyw};

use crate::conversion::{worker_http1_config_from_py, worker_http2_config_from_py};
use crate::workers::{serve_rth, serve_rth_ssl, serve_wth, serve_wth_ssl, WorkerConfig, WorkerSignal};

#[pyclass(frozen, module = "granian._granian")]
pub struct ASGIWorker {
    config: WorkerConfig,
}

impl ASGIWorker {
    serve_rth!(_serve_rth, handle);
    serve_rth!(_serve_rth_pyw, handle_pyw);
    serve_rth!(_serve_rth_ws, handle_ws);
    serve_rth!(_serve_rth_ws_pyw, handle_ws_pyw);
    serve_wth!(_serve_wth, handle);
    serve_wth!(_serve_wth_pyw, handle_pyw);
    serve_wth!(_serve_wth_ws, handle_ws);
    serve_wth!(_serve_wth_ws_pyw, handle_ws_pyw);
    serve_rth_ssl!(_serve_rth_ssl, handle);
    serve_rth_ssl!(_serve_rth_ssl_pyw, handle_pyw);
    serve_rth_ssl!(_serve_rth_ssl_ws, handle_ws);
    serve_rth_ssl!(_serve_rth_ssl_ws_pyw, handle_ws_pyw);
    serve_wth_ssl!(_serve_wth_ssl, handle);
    serve_wth_ssl!(_serve_wth_ssl_pyw, handle_pyw);
    serve_wth_ssl!(_serve_wth_ssl_ws, handle_ws);
    serve_wth_ssl!(_serve_wth_ssl_ws_pyw, handle_ws_pyw);
}

#[pymethods]
impl ASGIWorker {
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
            opt_enabled=true,
            ssl_enabled=false,
            ssl_cert=None,
            ssl_key=None
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
        opt_enabled: bool,
        ssl_enabled: bool,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
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
                opt_enabled,
                ssl_enabled,
                ssl_cert,
                ssl_key,
            ),
        })
    }

    fn serve_rth(
        &self,
        callback: PyObject,
        event_loop: &Bound<PyAny>,
        context: Bound<PyAny>,
        signal: Py<WorkerSignal>,
    ) {
        match (
            self.config.websockets_enabled,
            self.config.ssl_enabled,
            self.config.opt_enabled,
        ) {
            (false, false, true) => self._serve_rth(callback, event_loop, context, signal),
            (false, false, false) => self._serve_rth_pyw(callback, event_loop, context, signal),
            (true, false, true) => self._serve_rth_ws(callback, event_loop, context, signal),
            (true, false, false) => self._serve_rth_ws_pyw(callback, event_loop, context, signal),
            (false, true, true) => self._serve_rth_ssl(callback, event_loop, context, signal),
            (false, true, false) => self._serve_rth_ssl_pyw(callback, event_loop, context, signal),
            (true, true, true) => self._serve_rth_ssl_ws(callback, event_loop, context, signal),
            (true, true, false) => self._serve_rth_ssl_ws_pyw(callback, event_loop, context, signal),
        }
    }

    fn serve_wth(
        &self,
        callback: PyObject,
        event_loop: &Bound<PyAny>,
        context: Bound<PyAny>,
        signal: Py<WorkerSignal>,
    ) {
        match (
            self.config.websockets_enabled,
            self.config.ssl_enabled,
            self.config.opt_enabled,
        ) {
            (false, false, true) => self._serve_wth(callback, event_loop, context, signal),
            (false, false, false) => self._serve_wth_pyw(callback, event_loop, context, signal),
            (true, false, true) => self._serve_wth_ws(callback, event_loop, context, signal),
            (true, false, false) => self._serve_wth_ws_pyw(callback, event_loop, context, signal),
            (false, true, true) => self._serve_wth_ssl(callback, event_loop, context, signal),
            (false, true, false) => self._serve_wth_ssl_pyw(callback, event_loop, context, signal),
            (true, true, true) => self._serve_wth_ssl_ws(callback, event_loop, context, signal),
            (true, true, false) => self._serve_wth_ssl_ws_pyw(callback, event_loop, context, signal),
        }
    }
}
