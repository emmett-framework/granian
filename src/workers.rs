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
    pub http1_buffer_max: usize,
    pub websockets_enabled: bool
}

impl WorkerConfig {
    pub fn new(
        id: i32,
        socket_fd: i32,
        threads: usize,
        http1_buffer_max: usize,
        websockets_enabled: bool
    ) -> Self {
        Self {
            id,
            socket_fd,
            threads,
            http1_buffer_max,
            websockets_enabled
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
    }
}

macro_rules! serve_rth {
    ($func_name:ident, $target:expr) => {
        fn $func_name(
            &self,
            callback: PyObject,
            event_loop: &PyAny,
            context: &PyAny,
            signal_rx: PyObject
        ) {
            let rt = init_runtime_mt(self.config.threads);
            let rth = rt.handler();
            let tcp_listener = self.config.tcp_listener();
            let http1_buffer_max = self.config.http1_buffer_max;
            let callback_wrapper = CallbackWrapper::new(callback, event_loop, context);

            let worker_id = self.config.id;
            println!("Listener spawned: {}", worker_id);

            let svc_loop = run_until_complete(
                rt.handler(),
                event_loop,
                async move {
                    let service = make_service_fn(|socket: &AddrStream| {
                        let local_addr = socket.local_addr();
                        let remote_addr = socket.remote_addr();
                        let callback_wrapper = callback_wrapper.clone();
                        let rth = rth.clone();

                        async move {
                            Ok::<_, Infallible>(service_fn(move |req| {
                                let callback_wrapper = callback_wrapper.clone();
                                let rth = rth.clone();

                                async move {
                                    Ok::<_, Infallible>($target(
                                        rth,
                                        callback_wrapper,
                                        local_addr,
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
                    server.with_graceful_shutdown(async move {
                        Python::with_gil(|py| {
                            into_future(signal_rx.as_ref(py)).unwrap()
                        }).await.unwrap();
                    }).await.unwrap();
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
        fn $func_name(
            &self,
            callback: PyObject,
            event_loop: &PyAny,
            context: &PyAny,
            signal_rx: PyObject
        ) {
            let rtm = init_runtime_mt(1);

            let worker_id = self.config.id;
            println!("Process spawned: {}", worker_id);

            let callback_wrapper = CallbackWrapper::new(callback, event_loop, context);
            let mut workers = vec![];
            let (stx, srx) = tokio::sync::watch::channel(false);

            for thread_id in 0..self.config.threads {
                println!("Worker spawned: {}", thread_id);

                let tcp_listener = self.config.tcp_listener();
                let http1_buffer_max = self.config.http1_buffer_max.clone();
                let callback_wrapper = callback_wrapper.clone();
                let mut srx = srx.clone();

                workers.push(thread::spawn(move || {
                    let rt = init_runtime_st();
                    let rth = rt.handler();
                    let local = tokio::task::LocalSet::new();

                    block_on_local(rt, local, async move {
                        let service = make_service_fn(|socket: &AddrStream| {
                            let local_addr = socket.local_addr();
                            let remote_addr = socket.remote_addr();
                            let callback_wrapper = callback_wrapper.clone();
                            let rth = rth.clone();

                            async move {
                                Ok::<_, Infallible>(service_fn(move |req| {
                                    let callback_wrapper = callback_wrapper.clone();
                                    let rth = rth.clone();

                                    async move {
                                        Ok::<_, Infallible>($target(
                                            rth,
                                            callback_wrapper,
                                            local_addr,
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
                        server.with_graceful_shutdown(async move {
                            srx.changed().await.unwrap();
                        }).await.unwrap();
                    });
                }));
            };

            let main_loop = run_until_complete(
                rtm.handler(),
                event_loop,
                async move {
                    Python::with_gil(|py| {
                        into_future(signal_rx.as_ref(py)).unwrap()
                    }).await.unwrap();
                    stx.send(true).unwrap();
                    while let Some(worker) = workers.pop() {
                        worker.join().unwrap();
                    }
                    Ok(())
                }
            );

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

pub(crate) fn init_pymodule(module: &PyModule) -> PyResult<()> {
    module.add_class::<ASGIWorker>()?;
    module.add_class::<RSGIWorker>()?;

    Ok(())
}
