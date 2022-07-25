use hyper::{
    Server,
    server::conn::AddrStream,
    service::{make_service_fn, service_fn}
};
use pyo3::prelude::*;
use std::{convert::Infallible, process, thread};

use crate::{
    callbacks::CallbackWrapper,
    runtime::{
        block_on_local,
        init_runtime_mt,
        init_runtime_st,
        into_future,
        run_until_complete
    },
    workers::{WorkerConfig, WorkerExecutor, serve_rth, serve_wth}
};
use super::http::{handle_request, handle_request_with_ws};

#[pyclass(module="granian._granian")]
pub struct RSGIWorker {
    config: WorkerConfig
}

impl RSGIWorker {
    serve_rth!(_serve_rth, handle_request);
    serve_rth!(_serve_rth_ws, handle_request_with_ws);
    serve_wth!(_serve_wth, handle_request);
    serve_wth!(_serve_wth_ws, handle_request_with_ws);
}

#[pymethods]
impl RSGIWorker {
    #[new]
    #[args(socket_fd, threads="1", http1_buffer_max="65535")]
    fn new(
        worker_id: i32,
        socket_fd: i32,
        threads: usize,
        http1_buffer_max: usize,
        websockets_enabled: bool
    ) -> PyResult<Self> {
        Ok(Self {
            config: WorkerConfig::new(
                worker_id,
                socket_fd,
                threads,
                http1_buffer_max,
                websockets_enabled
            )
        })
    }

    fn serve_rth(
        &self,
        callback: PyObject,
        event_loop: &PyAny,
        context: &PyAny,
        signal_rx: PyObject
    ) {
        match self.config.websockets_enabled {
            false => self._serve_rth(callback, event_loop, context, signal_rx),
            true => self._serve_rth_ws(callback, event_loop, context, signal_rx)
        }
    }

    fn serve_wth(
        &self,
        callback: PyObject,
        event_loop: &PyAny,
        context: &PyAny,
        signal_rx: PyObject
    ) {
        match self.config.websockets_enabled {
            false => self._serve_wth(callback, event_loop, context, signal_rx),
            true => self._serve_wth_ws(callback, event_loop, context, signal_rx)
        }
    }
}
