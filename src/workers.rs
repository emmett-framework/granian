use pyo3::prelude::*;
use std::net::TcpListener;
#[cfg(unix)]
use std::os::unix::io::FromRawFd;
#[cfg(windows)]
use std::os::windows::io::FromRawSocket;

use super::asgi::serve::ASGIWorker;
use super::rsgi::serve::RSGIWorker;

pub(crate) struct WorkerConfig {
    pub id: i32,
    socket_fd: i32,
    pub threads: usize,
    pub http1_buffer_max: usize
}

impl WorkerConfig {
    pub fn new(
        id: i32,
        socket_fd: i32,
        threads: usize,
        http1_buffer_max: usize
    ) -> Self {
        Self {
            id,
            socket_fd,
            threads,
            http1_buffer_max
        }
    }

    #[cfg(unix)]
    pub fn tcp_listener(&self) -> TcpListener {
        unsafe {
            TcpListener::from_raw_fd(self.socket_fd)
        }
    }

    #[cfg(windows)]
    pub fn tcp_listener(&self) -> TcpListener {
        unsafe {
            TcpListener::from_raw_socket(self.socket_fd as u64)
        }
    }
}

pub(crate) fn worker_rt(config: &WorkerConfig) -> TcpListener {
    let mut tokio_builder = tokio::runtime::Builder::new_multi_thread();
    tokio_builder.worker_threads(config.threads);
    tokio_builder.enable_all();
    pyo3_asyncio::tokio::init(tokio_builder);

    config.tcp_listener()
}

pub(crate) fn build_pymodule(py: Python) -> PyResult<&PyModule> {
    let module = PyModule::new(py, "workers")?;

    module.add_class::<ASGIWorker>()?;
    module.add_class::<RSGIWorker>()?;

    Ok(module)
}
