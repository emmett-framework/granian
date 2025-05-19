use futures::FutureExt;
use pyo3::prelude::*;

use super::http::{handle, handle_ws};

use crate::callbacks::CallbackScheduler;
use crate::conversion::{worker_http1_config_from_py, worker_http2_config_from_py};
use crate::tcp::SocketHolder;
use crate::workers::{WorkerConfig, WorkerSignal, gen_serve_match, gen_serve_methods};

#[pyclass(frozen, module = "granian._granian")]
pub struct ASGIWorker {
    config: WorkerConfig,
}

impl ASGIWorker {
    gen_serve_methods!(handle);
    gen_serve_methods!(ws handle_ws);
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
            static_files=None,
            ssl_enabled=false,
            ssl_cert=None,
            ssl_key=None,
            ssl_key_password=None,
            ssl_ca=None,
            ssl_crl=vec![],
            ssl_client_verify=false,
        )
    )]
    fn new(
        py: Python,
        worker_id: i32,
        sock: Py<SocketHolder>,
        threads: usize,
        blocking_threads: usize,
        py_threads: usize,
        py_threads_idle_timeout: u64,
        backpressure: usize,
        http_mode: &str,
        http1_opts: Option<PyObject>,
        http2_opts: Option<PyObject>,
        websockets_enabled: bool,
        static_files: Option<(String, String, String)>,
        ssl_enabled: bool,
        ssl_cert: Option<String>,
        ssl_key: Option<String>,
        ssl_key_password: Option<String>,
        ssl_ca: Option<String>,
        ssl_crl: Vec<String>,
        ssl_client_verify: bool,
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
                static_files,
                ssl_enabled,
                ssl_cert,
                ssl_key,
                ssl_key_password,
                ssl_ca,
                ssl_crl,
                ssl_client_verify,
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
        gen_serve_match!(mtr self, py, callback, event_loop, signal);
    }

    fn serve_str(&self, callback: Py<CallbackScheduler>, event_loop: &Bound<PyAny>, signal: Py<WorkerSignal>) {
        gen_serve_match!(str self, callback, event_loop, signal);
    }

    fn serve_async<'p>(
        &self,
        callback: Py<CallbackScheduler>,
        event_loop: &Bound<'p, PyAny>,
        signal: Py<WorkerSignal>,
    ) -> Bound<'p, PyAny> {
        gen_serve_match!(fut self, callback, event_loop, signal)
    }
}
