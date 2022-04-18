use hyper::{
    Server,
    server::conn::AddrStream,
    service::{make_service_fn, service_fn}
};
use pyo3::prelude::*;
use std::{convert::Infallible, process};

use super::super::callbacks::CallbackWrapper;
use super::super::workers::{WorkerConfig, worker_rt};
use super::http::handle_request;

#[pyclass(module="granian.workers")]
pub struct RSGIWorker {
    config: WorkerConfig
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

    fn serve(&self, callback: PyObject, event_loop: &PyAny, context: &PyAny) {
        let tcp_listener = worker_rt(&self.config);
        let http1_buffer_max = self.config.http1_buffer_max;
        let callback_wrapper = CallbackWrapper::new(callback, event_loop, context);

        let worker_id = self.config.id;
        println!("Listener spawned: {}", worker_id);

        let svc_loop = pyo3_asyncio::tokio::run_until_complete(event_loop, async move {
            let service = make_service_fn(|socket: &AddrStream| {
                let remote_addr = socket.remote_addr();
                let callback_wrapper = callback_wrapper.clone();

                async move {
                    Ok::<_, Infallible>(service_fn(move |req| {
                        let callback_wrapper = callback_wrapper.clone();

                        async move {
                            Ok::<_, Infallible>(handle_request(
                                callback_wrapper, remote_addr, req
                            ).await.unwrap())
                        }
                    }))
                }
            });

            let server = Server::from_tcp(tcp_listener).unwrap()
                .http1_max_buf_size(http1_buffer_max)
                .serve(service);
            server.await.unwrap();
            Ok(())
        });

        match svc_loop {
            Ok(_) => {}
            Err(err) => {
                println!("err: {}", err);
                process::exit(1);
            }
        };
    }
}
