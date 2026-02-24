use pyo3::prelude::*;

use super::http::{handle, handle_ws};

use crate::callbacks::CallbackScheduler;
use crate::conversion::{worker_http1_config_from_py, worker_http2_config_from_py, worker_static_files_config_from_py};
use crate::net::SocketHolder;
use crate::serve::gen_serve_match;
use crate::workers::{WorkerConfig, WorkerSignal};

#[pyclass(frozen, module = "granian._granian")]
pub struct RSGIWorker {
    config: WorkerConfig,
}

#[pymethods]
impl RSGIWorker {
    #[new]
    #[pyo3(
        signature = (
            worker_id,
            sock,
            ipc,
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
            ssl_protocol_min="1.3",
            ssl_ca=None,
            ssl_crl=vec![],
            ssl_client_verify=false,
            metrics=(None, None),
        )
    )]
    fn new(
        py: Python,
        worker_id: i32,
        sock: Py<SocketHolder>,
        ipc: Option<Py<crate::ipc::IPCSenderHandle>>,
        threads: usize,
        blocking_threads: usize,
        py_threads: usize,
        py_threads_idle_timeout: u64,
        backpressure: usize,
        http_mode: &str,
        http1_opts: Option<Py<PyAny>>,
        http2_opts: Option<Py<PyAny>>,
        websockets_enabled: bool,
        static_files: Option<Py<PyAny>>,
        ssl_enabled: bool,
        ssl_cert: Option<String>,
        ssl_key: Option<String>,
        ssl_key_password: Option<String>,
        ssl_protocol_min: &str,
        ssl_ca: Option<String>,
        ssl_crl: Vec<String>,
        ssl_client_verify: bool,
        metrics: (Option<u64>, Option<Py<crate::metrics::MetricsAggregator>>),
    ) -> PyResult<Self> {
        Ok(Self {
            config: WorkerConfig::new(
                worker_id,
                sock,
                ipc,
                threads,
                blocking_threads,
                py_threads,
                py_threads_idle_timeout,
                backpressure,
                http_mode,
                worker_http1_config_from_py(py, http1_opts)?,
                worker_http2_config_from_py(py, http2_opts)?,
                websockets_enabled,
                worker_static_files_config_from_py(py, static_files)?,
                ssl_enabled,
                ssl_cert,
                ssl_key,
                ssl_key_password,
                ssl_protocol_min,
                ssl_ca,
                ssl_crl,
                ssl_client_verify,
                metrics,
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
        gen_serve_match!(
            crate::serve::serve_mt,
            WorkerAcceptorTcpPlain,
            WorkerAcceptorTcpTls,
            self,
            py,
            callback,
            event_loop,
            signal,
            handle,
            handle_ws
        );
    }

    fn serve_str(&self, callback: Py<CallbackScheduler>, event_loop: &Bound<PyAny>, signal: Py<WorkerSignal>) {
        gen_serve_match!(
            crate::serve::serve_st,
            WorkerAcceptorTcpPlain,
            WorkerAcceptorTcpTls,
            self,
            (),
            callback,
            event_loop,
            signal,
            handle,
            handle_ws
        );
    }

    fn serve_async<'p>(
        &self,
        callback: Py<CallbackScheduler>,
        event_loop: &Bound<'p, PyAny>,
        signal: Py<WorkerSignal>,
    ) -> Bound<'p, PyAny> {
        gen_serve_match!(
            crate::serve::serve_fut,
            WorkerAcceptorTcpPlain,
            WorkerAcceptorTcpTls,
            self,
            (),
            callback,
            event_loop,
            signal,
            handle,
            handle_ws
        )
    }

    #[cfg(unix)]
    fn serve_mtr_uds(
        &self,
        py: Python,
        callback: Py<CallbackScheduler>,
        event_loop: &Bound<PyAny>,
        signal: Py<WorkerSignal>,
    ) {
        gen_serve_match!(
            crate::serve::serve_mt_uds,
            WorkerAcceptorUdsPlain,
            WorkerAcceptorUdsTls,
            self,
            py,
            callback,
            event_loop,
            signal,
            handle,
            handle_ws
        );
    }

    #[cfg(unix)]
    fn serve_str_uds(&self, callback: Py<CallbackScheduler>, event_loop: &Bound<PyAny>, signal: Py<WorkerSignal>) {
        gen_serve_match!(
            crate::serve::serve_st_uds,
            WorkerAcceptorUdsPlain,
            WorkerAcceptorUdsTls,
            self,
            (),
            callback,
            event_loop,
            signal,
            handle,
            handle_ws
        );
    }

    #[cfg(unix)]
    fn serve_async_uds<'p>(
        &self,
        callback: Py<CallbackScheduler>,
        event_loop: &Bound<'p, PyAny>,
        signal: Py<WorkerSignal>,
    ) -> Bound<'p, PyAny> {
        gen_serve_match!(
            crate::serve::serve_fut_uds,
            WorkerAcceptorUdsPlain,
            WorkerAcceptorUdsTls,
            self,
            (),
            callback,
            event_loop,
            signal,
            handle,
            handle_ws
        )
    }
}
