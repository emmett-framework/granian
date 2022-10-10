use pyo3::prelude::*;
use std::net::TcpListener;

#[cfg(unix)]
use std::os::unix::io::FromRawFd;
#[cfg(windows)]
use std::os::windows::io::FromRawSocket;

use super::asgi::serve::ASGIWorker;
use super::rsgi::serve::RSGIWorker;
use super::tls::{load_certs as tls_load_certs, load_private_key as tls_load_pkey};

pub(crate) struct WorkerConfig {
    pub id: i32,
    socket_fd: i32,
    pub threads: usize,
    pub http_mode: String,
    pub http1_buffer_max: usize,
    pub websockets_enabled: bool,
    pub ssl_enabled: bool,
    ssl_cert: Option<String>,
    ssl_key: Option<String>
}

impl WorkerConfig {
    pub fn new(
        id: i32,
        socket_fd: i32,
        threads: usize,
        http_mode: String,
        http1_buffer_max: usize,
        websockets_enabled: bool,
        ssl_enabled: bool,
        ssl_cert: Option<String>,
        ssl_key: Option<String>
    ) -> Self {
        Self {
            id,
            socket_fd,
            threads,
            http_mode,
            http1_buffer_max,
            websockets_enabled,
            ssl_enabled,
            ssl_cert,
            ssl_key
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

    pub fn tls_cfg(&self) -> tokio_rustls::rustls::ServerConfig {
        let mut cfg = tokio_rustls::rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(
                tls_load_certs(&self.ssl_cert.clone().unwrap()[..]).unwrap(),
                tls_load_pkey(&self.ssl_key.clone().unwrap()[..]).unwrap()
            )
            .unwrap();
        cfg.alpn_protocols = match &self.http_mode[..] {
            "1" => vec![b"http/1.1".to_vec()],
            "2" => vec![b"h2".to_vec()],
            _ => vec![b"h2".to_vec(), b"http/1.1".to_vec()]
        };
        cfg
    }
}

// pub(crate) struct Worker<R>
// where R: Future<Output=Response<Body>> + Send
// {
//     config: WorkerConfig,
//     handler: fn(
//         crate::callbacks::CallbackWrapper,
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

macro_rules! build_service {
    ($callback_wrapper:expr, $rt:expr, $target:expr) => {
        hyper::service::make_service_fn(|socket: &hyper::server::conn::AddrStream| {
            let local_addr = socket.local_addr();
            let remote_addr = socket.remote_addr();
            let callback_wrapper = $callback_wrapper.clone();
            let rth = $rt.clone();

            async move {
                Ok::<_, std::convert::Infallible>(hyper::service::service_fn(move |req| {
                    let callback_wrapper = callback_wrapper.clone();
                    let rth = rth.clone();

                    async move {
                        Ok::<_, std::convert::Infallible>($target(
                            rth,
                            callback_wrapper,
                            local_addr,
                            remote_addr,
                            req,
                            "http"
                        ).await)
                    }
                }))
            }
        })
    };
}

macro_rules! build_service_ssl {
    ($callback_wrapper:expr, $rt:expr, $target:expr) => {
        hyper::service::make_service_fn(|stream: &crate::tls::TlsAddrStream| {
            let (socket, _) = stream.get_ref();
            let local_addr = socket.local_addr();
            let remote_addr = socket.remote_addr();
            let callback_wrapper = $callback_wrapper.clone();
            let rth = $rt.clone();

            async move {
                Ok::<_, std::convert::Infallible>(hyper::service::service_fn(move |req| {
                    let callback_wrapper = callback_wrapper.clone();
                    let rth = rth.clone();

                    async move {
                        Ok::<_, std::convert::Infallible>($target(
                            rth,
                            callback_wrapper,
                            local_addr,
                            remote_addr,
                            req,
                            "https"
                        ).await)
                    }
                }))
            }
        })
    };
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
            pyo3_log::init();
            let rt = crate::runtime::init_runtime_mt(self.config.threads);
            let rth = rt.handler();
            let tcp_listener = self.config.tcp_listener();
            let http1_only = self.config.http_mode == "1";
            let http2_only = self.config.http_mode == "2";
            let http1_buffer_max = self.config.http1_buffer_max.clone();
            let callback_wrapper = crate::callbacks::CallbackWrapper::new(
                callback, event_loop, context
            );

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let svc_loop = crate::runtime::run_until_complete(
                rt.handler(),
                event_loop,
                async move {
                    let service = crate::workers::build_service!(
                        callback_wrapper, rth, $target
                    );
                    let server = hyper::Server::from_tcp(tcp_listener).unwrap()
                        .http1_only(http1_only)
                        .http2_only(http2_only)
                        .http1_max_buf_size(http1_buffer_max)
                        .serve(service);
                    server.with_graceful_shutdown(async move {
                        Python::with_gil(|py| {
                            crate::runtime::into_future(signal_rx.as_ref(py)).unwrap()
                        }).await.unwrap();
                    }).await.unwrap();
                    log::info!("Stopping worker-{}", worker_id);
                    Ok(())
                }
            );

            match svc_loop {
                Ok(_) => {}
                Err(err) => {
                    log::error!("{}", err);
                    std::process::exit(1);
                }
            };
        }
    };
}

macro_rules! serve_rth_ssl {
    ($func_name:ident, $target:expr) => {
        fn $func_name(
            &self,
            callback: PyObject,
            event_loop: &PyAny,
            context: &PyAny,
            signal_rx: PyObject
        ) {
            pyo3_log::init();
            let rt = crate::runtime::init_runtime_mt(self.config.threads);
            let rth = rt.handler();
            let tcp_listener = self.config.tcp_listener();
            let http1_only = self.config.http_mode == "1";
            let http2_only = self.config.http_mode == "2";
            let http1_buffer_max = self.config.http1_buffer_max.clone();
            let tls_cfg = self.config.tls_cfg();
            let callback_wrapper = crate::callbacks::CallbackWrapper::new(
                callback, event_loop, context
            );

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let svc_loop = crate::runtime::run_until_complete(
                rt.handler(),
                event_loop,
                async move {
                    let service = crate::workers::build_service_ssl!(
                        callback_wrapper, rth, $target
                    );
                    let server = hyper::Server::builder(
                        crate::tls::tls_listen(
                            std::sync::Arc::new(tls_cfg), tcp_listener
                        )
                    )
                        .http1_only(http1_only)
                        .http2_only(http2_only)
                        .http1_max_buf_size(http1_buffer_max)
                        .serve(service);
                    server.with_graceful_shutdown(async move {
                        Python::with_gil(|py| {
                            crate::runtime::into_future(signal_rx.as_ref(py)).unwrap()
                        }).await.unwrap();
                    }).await.unwrap();
                    log::info!("Stopping worker-{}", worker_id);
                    Ok(())
                }
            );

            match svc_loop {
                Ok(_) => {}
                Err(err) => {
                    log::error!("{}", err);
                    std::process::exit(1);
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
            pyo3_log::init();
            let rtm = crate::runtime::init_runtime_mt(1);

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let callback_wrapper = crate::callbacks::CallbackWrapper::new(
                callback, event_loop, context
            );
            let mut workers = vec![];
            let (stx, srx) = tokio::sync::watch::channel(false);

            for thread_id in 0..self.config.threads {
                log::info!("Started worker-{} runtime-{}", worker_id, thread_id);

                let tcp_listener = self.config.tcp_listener();
                let http1_only = self.config.http_mode == "1";
                let http2_only = self.config.http_mode == "2";
                let http1_buffer_max = self.config.http1_buffer_max.clone();
                let callback_wrapper = callback_wrapper.clone();
                let mut srx = srx.clone();

                workers.push(std::thread::spawn(move || {
                    let rt = crate::runtime::init_runtime_st();
                    let rth = rt.handler();
                    let local = tokio::task::LocalSet::new();

                    crate::runtime::block_on_local(rt, local, async move {
                        let service = crate::workers::build_service!(
                            callback_wrapper, rth, $target
                        );
                        let server = hyper::Server::from_tcp(tcp_listener).unwrap()
                            .executor(crate::workers::WorkerExecutor)
                            .http1_only(http1_only)
                            .http2_only(http2_only)
                            .http1_max_buf_size(http1_buffer_max)
                            .serve(service);
                        server.with_graceful_shutdown(async move {
                            srx.changed().await.unwrap();
                        }).await.unwrap();
                        log::info!("Stopping worker-{} runtime-{}", worker_id, thread_id);
                    });
                }));
            };

            let main_loop = crate::runtime::run_until_complete(
                rtm.handler(),
                event_loop,
                async move {
                    Python::with_gil(|py| {
                        crate::runtime::into_future(signal_rx.as_ref(py)).unwrap()
                    }).await.unwrap();
                    stx.send(true).unwrap();
                    log::info!("Stopping worker-{}", worker_id);
                    while let Some(worker) = workers.pop() {
                        worker.join().unwrap();
                    }
                    Ok(())
                }
            );

            match main_loop {
                Ok(_) => {}
                Err(err) => {
                    log::error!("{}", err);
                    std::process::exit(1);
                }
            };
        }
    };
}

macro_rules! serve_wth_ssl {
    ($func_name: ident, $target:expr) => {
        fn $func_name(
            &self,
            callback: PyObject,
            event_loop: &PyAny,
            context: &PyAny,
            signal_rx: PyObject
        ) {
            pyo3_log::init();
            let rtm = crate::runtime::init_runtime_mt(1);

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let callback_wrapper = crate::callbacks::CallbackWrapper::new(
                callback, event_loop, context
            );
            let mut workers = vec![];
            let (stx, srx) = tokio::sync::watch::channel(false);

            for thread_id in 0..self.config.threads {
                log::info!("Started worker-{} runtime-{}", worker_id, thread_id);

                let tcp_listener = self.config.tcp_listener();
                let http1_only = self.config.http_mode == "1";
                let http2_only = self.config.http_mode == "2";
                let http1_buffer_max = self.config.http1_buffer_max.clone();
                let tls_cfg = self.config.tls_cfg();
                let callback_wrapper = callback_wrapper.clone();
                let mut srx = srx.clone();

                workers.push(std::thread::spawn(move || {
                    let rt = crate::runtime::init_runtime_st();
                    let rth = rt.handler();
                    let local = tokio::task::LocalSet::new();

                    crate::runtime::block_on_local(rt, local, async move {
                        let service = crate::workers::build_service_ssl!(
                            callback_wrapper, rth, $target
                        );
                        let server = hyper::Server::builder(
                            crate::tls::tls_listen(
                                std::sync::Arc::new(tls_cfg), tcp_listener
                            )
                        )
                            .executor(crate::workers::WorkerExecutor)
                            .http1_only(http1_only)
                            .http2_only(http2_only)
                            .http1_max_buf_size(http1_buffer_max)
                            .serve(service);
                        server.with_graceful_shutdown(async move {
                            srx.changed().await.unwrap();
                        }).await.unwrap();
                        log::info!("Stopping worker-{} runtime-{}", worker_id, thread_id);
                    });
                }));
            };

            let main_loop = crate::runtime::run_until_complete(
                rtm.handler(),
                event_loop,
                async move {
                    Python::with_gil(|py| {
                        crate::runtime::into_future(signal_rx.as_ref(py)).unwrap()
                    }).await.unwrap();
                    stx.send(true).unwrap();
                    log::info!("Stopping worker-{}", worker_id);
                    while let Some(worker) = workers.pop() {
                        worker.join().unwrap();
                    }
                    Ok(())
                }
            );

            match main_loop {
                Ok(_) => {}
                Err(err) => {
                    log::error!("{}", err);
                    std::process::exit(1);
                }
            };
        }
    };
}

pub(crate) use build_service;
pub(crate) use build_service_ssl;
pub(crate) use serve_rth;
pub(crate) use serve_wth;
pub(crate) use serve_rth_ssl;
pub(crate) use serve_wth_ssl;

pub(crate) fn init_pymodule(module: &PyModule) -> PyResult<()> {
    module.add_class::<ASGIWorker>()?;
    module.add_class::<RSGIWorker>()?;

    Ok(())
}
