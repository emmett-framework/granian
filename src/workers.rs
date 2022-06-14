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

// pub(crate) struct Worker<R>
// where R: Future<Output=Response<Body>> + Send
// {
//     config: WorkerConfig,
//     handler: fn(
//         CallbackWrapper,
//         SocketAddr,
//         Request<Body>
//     ) -> R
// }

#[derive(Clone, Copy)]
pub(crate) struct WorkerExecutor;

impl<F> hyper::rt::Executor<F> for WorkerExecutor
where
    F: std::future::Future + 'static
{
    fn execute(&self, fut: F) {
        tokio::task::spawn_local(fut);
        // current_runtime().spawn(fut);
    }
}

pub(crate) fn worker_rt(config: &WorkerConfig) -> TcpListener {
    let mut tokio_builder = tokio::runtime::Builder::new_multi_thread();
    tokio_builder.worker_threads(config.threads);
    tokio_builder.enable_all();
    pyo3_asyncio::tokio::init(tokio_builder);

    config.tcp_listener()
}

macro_rules! serve_rth {
    ($func_name:ident, $target:expr) => {
        fn $func_name(&self, callback: PyObject, event_loop: &PyAny, context: &PyAny) {
            let tcp_listener = worker_rt(&self.config);
            let http1_buffer_max = self.config.http1_buffer_max;
            let callback_wrapper = CallbackWrapper::new(callback, event_loop, context);

            let worker_id = self.config.id;
            println!("Listener spawned: {}", worker_id);

            let svc_loop = pyo3_asyncio::tokio::run_until_complete(
                event_loop,
                async move {
                    let service = make_service_fn(|socket: &AddrStream| {
                        let remote_addr = socket.remote_addr();
                        let callback_wrapper = callback_wrapper.clone();

                        async move {
                            Ok::<_, Infallible>(service_fn(move |req| {
                                let callback_wrapper = callback_wrapper.clone();

                                async move {
                                    Ok::<_, Infallible>($target(
                                        ThreadIsolation::Runtime,
                                        callback_wrapper,
                                        remote_addr,
                                        req
                                    ).await)
                                }
                            }))
                        }
                    });

                    let server = Server::from_tcp(tcp_listener).unwrap()
                        .http1_max_buf_size(http1_buffer_max)
                        .serve(service);
                    server.await.unwrap();
                    Ok(())
                }
            );

            match svc_loop {
                Ok(_) => {}
                Err(err) => {
                    println!("err: {}", err);
                    process::exit(1);
                }
            };
        }
    };
}

macro_rules! serve_wth {
    ($func_name: ident, $target:expr) => {
        fn $func_name(&self, callback: PyObject, event_loop: &PyAny, context: &PyAny) {
            init_runtime();

            let worker_id = self.config.id;
            println!("Process spawned: {}", worker_id);

            let callback_wrapper = CallbackWrapper::new(callback, event_loop, context);
            let mut workers = vec![];

            for thread_id in 0..self.config.threads {
                println!("Worker spawned: {}", thread_id);

                let tcp_listener = self.config.tcp_listener();
                let http1_buffer_max = self.config.http1_buffer_max.clone();
                let callback_wrapper = callback_wrapper.clone();

                workers.push(thread::spawn(move || {
                    init_runtime();
                    let local = tokio::task::LocalSet::new();

                    block_on_local(local, async move {
                        let service = make_service_fn(|socket: &AddrStream| {
                            let remote_addr = socket.remote_addr();
                            let callback_wrapper = callback_wrapper.clone();

                            async move {
                                Ok::<_, Infallible>(service_fn(move |req| {
                                    let callback_wrapper = callback_wrapper.clone();

                                    async move {
                                        Ok::<_, Infallible>(handle_request(
                                            ThreadIsolation::Worker,
                                            callback_wrapper,
                                            remote_addr,
                                            req
                                        ).await)
                                    }
                                }))
                            }
                        });

                        let server = Server::from_tcp(tcp_listener).unwrap()
                            .executor(WorkerExecutor)
                            .http1_max_buf_size(http1_buffer_max)
                            .serve(service);
                        server.await.unwrap();
                    });
                }));
            };

            let main_loop = run_until_complete(event_loop, async move {
                while let Some(worker) = workers.pop() {
                    worker.join().unwrap();
                }
                Ok(())
            });

            match main_loop {
                Ok(_) => {}
                Err(err) => {
                    println!("err: {}", err);
                    process::exit(1);
                }
            };
        }
    };
}

pub(crate) use serve_rth;
pub(crate) use serve_wth;

pub(crate) fn build_pymodule(py: Python) -> PyResult<&PyModule> {
    let module = PyModule::new(py, "workers")?;

    module.add_class::<ASGIWorker>()?;
    module.add_class::<RSGIWorker>()?;

    Ok(module)
}
