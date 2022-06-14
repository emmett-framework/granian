use hyper::{
    Server,
    server::conn::AddrStream,
    service::{make_service_fn, service_fn}
};
use pyo3::prelude::*;
use std::{convert::Infallible, process, thread};

use super::super::{
    callbacks::CallbackWrapper,
    runtime::{
        ThreadIsolation,
        block_on_local,
        init_runtime,
        run_until_complete
    },
    workers::{WorkerConfig, WorkerExecutor, serve_rth, serve_wth, worker_rt}
};
use super::http::handle_request;

#[pyclass(module="granian.workers")]
pub struct RSGIWorker {
    config: WorkerConfig
}

impl RSGIWorker {
    serve_rth!(_serve_rth, handle_request);
    serve_wth!(_serve_wth, handle_request);
}

#[pymethods]
impl RSGIWorker {
    #[new]
    #[args(socket_fd, threads="1", http1_buffer_max="65535")]
    fn new(
        worker_id: i32,
        socket_fd: i32,
        threads: usize,
        http1_buffer_max: usize
    ) -> PyResult<Self> {
        Ok(Self {
            config: WorkerConfig::new(
                worker_id,
                socket_fd,
                threads,
                http1_buffer_max
            )
        })
    }

    fn serve_rth(&self, callback: PyObject, event_loop: &PyAny, context: &PyAny) {
       self._serve_rth(callback, event_loop, context)
    }

    fn serve_wth(&self, callback: PyObject, event_loop: &PyAny, context: &PyAny) {
        self._serve_wth(callback, event_loop, context)
    }
}
