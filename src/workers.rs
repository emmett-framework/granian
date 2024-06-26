use pyo3::prelude::*;
use std::net::TcpListener;
use std::sync::Mutex;

#[cfg(unix)]
use std::os::unix::io::FromRawFd;
#[cfg(windows)]
use std::os::windows::io::FromRawSocket;

use super::asgi::serve::ASGIWorker;
use super::rsgi::serve::RSGIWorker;
use super::tls::{load_certs as tls_load_certs, load_private_key as tls_load_pkey};
use super::wsgi::serve::WSGIWorker;

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct WorkerSignal {
    pub rx: Mutex<Option<tokio::sync::watch::Receiver<bool>>>,
    tx: tokio::sync::watch::Sender<bool>,
}

#[pymethods]
impl WorkerSignal {
    #[new]
    fn new() -> Self {
        let (tx, rx) = tokio::sync::watch::channel(false);
        Self {
            rx: Mutex::new(Some(rx)),
            tx,
        }
    }

    fn set(&self) {
        let _ = self.tx.send(true);
    }
}

#[derive(Clone)]
pub(crate) struct HTTP1Config {
    pub keep_alive: bool,
    pub max_buffer_size: usize,
    pub pipeline_flush: bool,
}

#[derive(Clone)]
pub(crate) struct HTTP2Config {
    pub adaptive_window: bool,
    pub initial_connection_window_size: u32,
    pub initial_stream_window_size: u32,
    pub keep_alive_interval: Option<core::time::Duration>,
    pub keep_alive_timeout: core::time::Duration,
    pub max_concurrent_streams: u32,
    pub max_frame_size: u32,
    pub max_headers_size: u32,
    pub max_send_buffer_size: usize,
}

pub(crate) struct WorkerConfig {
    pub id: i32,
    socket_fd: i32,
    pub threads: usize,
    pub blocking_threads: usize,
    pub backpressure: usize,
    pub http_mode: String,
    pub http1_opts: HTTP1Config,
    pub http2_opts: HTTP2Config,
    pub websockets_enabled: bool,
    pub opt_enabled: bool,
    pub ssl_enabled: bool,
    ssl_cert: Option<String>,
    ssl_key: Option<String>,
}

impl WorkerConfig {
    pub fn new(
        id: i32,
        socket_fd: i32,
        threads: usize,
        blocking_threads: usize,
        backpressure: usize,
        http_mode: &str,
        http1_opts: HTTP1Config,
        http2_opts: HTTP2Config,
        websockets_enabled: bool,
        opt_enabled: bool,
        ssl_enabled: bool,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
    ) -> Self {
        Self {
            id,
            socket_fd,
            threads,
            blocking_threads,
            backpressure,
            http_mode: http_mode.into(),
            http1_opts,
            http2_opts,
            websockets_enabled,
            opt_enabled,
            ssl_enabled,
            ssl_cert: ssl_cert.map(std::convert::Into::into),
            ssl_key: ssl_key.map(std::convert::Into::into),
        }
    }

    #[cfg(unix)]
    pub fn tcp_listener(&self) -> TcpListener {
        let listener = unsafe { TcpListener::from_raw_fd(self.socket_fd) };
        let _ = listener.set_nonblocking(true);
        listener
    }

    #[cfg(windows)]
    pub fn tcp_listener(&self) -> TcpListener {
        let listener = unsafe { TcpListener::from_raw_socket(self.socket_fd as u64) };
        let _ = listener.set_nonblocking(true);
        listener
    }

    pub fn tls_cfg(&self) -> tls_listener::rustls::rustls::ServerConfig {
        let mut cfg = tls_listener::rustls::rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                tls_load_certs(self.ssl_cert.clone().unwrap()).unwrap(),
                tls_load_pkey(self.ssl_key.clone().unwrap()).unwrap(),
            )
            .unwrap();
        cfg.alpn_protocols = match &self.http_mode[..] {
            "1" => vec![b"http/1.1".to_vec()],
            "2" => vec![b"h2".to_vec()],
            _ => vec![b"h2".to_vec(), b"http/1.1".to_vec()],
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

impl WorkerExecutor {
    pub fn new() -> Self {
        Self {}
    }
}

impl<F> hyper::rt::Executor<F> for WorkerExecutor
where
    F: std::future::Future + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn_local(fut);
    }
}

macro_rules! build_service {
    ($local_addr:expr, $remote_addr:expr, $callback_wrapper:expr, $rt:expr, $target:expr, $proto:expr) => {
        hyper::service::service_fn(move |request: crate::http::HTTPRequest| {
            let callback_wrapper = $callback_wrapper.clone();
            let rth = $rt.clone();

            async move {
                Ok::<_, anyhow::Error>($target(rth, callback_wrapper, $local_addr, $remote_addr, request, $proto).await)
            }
        })
    };
}

macro_rules! handle_connection_loop {
    ($tcp_listener:expr, $quit_signal:expr, $backpressure:expr, $inner:expr) => {
        let tcp_listener = tokio::net::TcpListener::from_std($tcp_listener).unwrap();
        let local_addr = tcp_listener.local_addr().unwrap();
        let mut accept_loop = true;
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new($backpressure));

        while accept_loop {
            let semaphore = semaphore.clone();
            tokio::select! {
                (permit, Ok((stream, remote_addr))) = async {
                    let permit = semaphore.acquire_owned().await.unwrap();
                    (permit, tcp_listener.accept().await)
                } => {
                    $inner(local_addr, remote_addr, stream, permit)
                },
                _ = $quit_signal => {
                    accept_loop = false;
                }
            }
        }
    };
}

macro_rules! handle_tls_loop {
    ($tcp_listener:expr, $tls_config:expr, $quit_signal:expr, $backpressure:expr, $inner:expr) => {
        let (mut tls_listener, local_addr) = crate::tls::tls_listener($tls_config.into(), $tcp_listener).unwrap();
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new($backpressure));
        let mut accept_loop = true;

        while accept_loop {
            let semaphore = semaphore.clone();
            tokio::select! {
                (permit, accept) = async {
                    let permit = semaphore.acquire_owned().await.unwrap();
                    (permit, tls_listener.accept().await)
                } => {
                    match accept {
                        Ok((stream, remote_addr)) => {
                            $inner(local_addr, remote_addr, stream, permit)
                        },
                        Err(err) => {
                            log::info!("TLS handshake failed with {:?}", err);
                        }
                    }
                },
                _ = $quit_signal => {
                    accept_loop = false;
                }
            }
        }
    };
}

macro_rules! handle_connection_http1 {
    ($rth:expr, $callback:expr, $spawner:expr, $stream_wrapper:expr, $proto:expr, $http_opts:expr, $target:expr) => {
        |local_addr, remote_addr, stream, permit| {
            let rth = $rth.clone();
            let callback_wrapper = $callback.clone();
            $spawner(async move {
                let svc =
                    crate::workers::build_service!(local_addr, remote_addr, callback_wrapper, rth, $target, $proto);
                let mut conn = hyper::server::conn::http1::Builder::new();
                conn.keep_alive($http_opts.keep_alive);
                conn.max_buf_size($http_opts.max_buffer_size);
                conn.pipeline_flush($http_opts.pipeline_flush);
                let _ = conn.serve_connection($stream_wrapper(stream), svc).await;
                drop(permit);
            });
        }
    };
}

macro_rules! handle_connection_http1_upgrades {
    ($rth:expr, $callback:expr, $spawner:expr, $stream_wrapper:expr, $proto:expr, $http_opts:expr, $target:expr) => {
        |local_addr, remote_addr, stream, permit| {
            let rth = $rth.clone();
            let callback_wrapper = $callback.clone();
            $spawner(async move {
                let svc =
                    crate::workers::build_service!(local_addr, remote_addr, callback_wrapper, rth, $target, $proto);
                let mut conn = hyper::server::conn::http1::Builder::new();
                conn.keep_alive($http_opts.keep_alive);
                conn.max_buf_size($http_opts.max_buffer_size);
                conn.pipeline_flush($http_opts.pipeline_flush);
                let _ = conn
                    .serve_connection($stream_wrapper(stream), svc)
                    .with_upgrades()
                    .await;
                drop(permit);
            });
        }
    };
}

macro_rules! handle_connection_http2 {
    ($rth:expr, $callback:expr, $spawner:expr, $executor_builder:expr, $stream_wrapper:expr, $proto:expr, $http_opts:expr, $target:expr) => {
        |local_addr, remote_addr, stream, permit| {
            let rth = $rth.clone();
            let callback_wrapper = $callback.clone();
            $spawner(async move {
                let svc =
                    crate::workers::build_service!(local_addr, remote_addr, callback_wrapper, rth, $target, $proto);
                let mut conn = hyper::server::conn::http2::Builder::new($executor_builder());
                conn.adaptive_window($http_opts.adaptive_window);
                conn.initial_connection_window_size($http_opts.initial_connection_window_size);
                conn.initial_stream_window_size($http_opts.initial_stream_window_size);
                conn.keep_alive_interval($http_opts.keep_alive_interval);
                conn.keep_alive_timeout($http_opts.keep_alive_timeout);
                conn.max_concurrent_streams($http_opts.max_concurrent_streams);
                conn.max_frame_size($http_opts.max_frame_size);
                conn.max_header_list_size($http_opts.max_headers_size);
                conn.max_send_buf_size($http_opts.max_send_buffer_size);
                let _ = conn.serve_connection($stream_wrapper(stream), svc).await;
                drop(permit);
            });
        }
    };
}

macro_rules! handle_connection_httpa {
    ($rth:expr, $callback:expr, $spawner:expr, $executor_builder:expr, $conn_method:ident, $stream_wrapper:expr, $proto:expr, $http1_opts:expr, $http2_opts:expr, $target:expr) => {
        |local_addr, remote_addr, stream, permit| {
            let rth = $rth.clone();
            let callback_wrapper = $callback.clone();
            $spawner(async move {
                let svc =
                    crate::workers::build_service!(local_addr, remote_addr, callback_wrapper, rth, $target, $proto);
                let mut conn = hyper_util::server::conn::auto::Builder::new($executor_builder());
                conn.http1().keep_alive($http1_opts.keep_alive);
                conn.http1().max_buf_size($http1_opts.max_buffer_size);
                conn.http1().pipeline_flush($http1_opts.pipeline_flush);
                conn.http2().adaptive_window($http2_opts.adaptive_window);
                conn.http2()
                    .initial_connection_window_size($http2_opts.initial_connection_window_size);
                conn.http2()
                    .initial_stream_window_size($http2_opts.initial_stream_window_size);
                conn.http2().keep_alive_interval($http2_opts.keep_alive_interval);
                conn.http2().keep_alive_timeout($http2_opts.keep_alive_timeout);
                conn.http2().max_concurrent_streams($http2_opts.max_concurrent_streams);
                conn.http2().max_frame_size($http2_opts.max_frame_size);
                conn.http2().max_header_list_size($http2_opts.max_headers_size);
                conn.http2().max_send_buf_size($http2_opts.max_send_buffer_size);
                let _ = conn.$conn_method($stream_wrapper(stream), svc).await;
                drop(permit);
            });
        }
    };
}

macro_rules! serve_rth {
    ($func_name:ident, $target:expr) => {
        fn $func_name(
            &self,
            callback: PyObject,
            event_loop: &Bound<PyAny>,
            context: Bound<PyAny>,
            signal: Py<crate::workers::WorkerSignal>,
        ) {
            pyo3_log::init();
            let rt = crate::runtime::init_runtime_mt(
                self.config.threads,
                self.config.blocking_threads,
                std::sync::Arc::new(event_loop.clone().unbind()),
            );
            let rth = rt.handler();
            let tcp_listener = self.config.tcp_listener();

            let http_mode = self.config.http_mode.clone();
            let http_upgrades = self.config.websockets_enabled;
            let http1_opts = self.config.http1_opts.clone();
            let http2_opts = self.config.http2_opts.clone();
            let backpressure = self.config.backpressure.clone();
            let callback_wrapper = crate::callbacks::CallbackWrapper::new(callback, event_loop.clone(), context);
            let mut pyrx = signal.get().rx.lock().unwrap().take().unwrap();

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let svc_loop = crate::runtime::run_until_complete(rt.handler(), event_loop.clone(), async move {
                match (&http_mode[..], http_upgrades) {
                    ("auto", true) => {
                        crate::workers::handle_connection_loop!(
                            tcp_listener,
                            pyrx.changed(),
                            backpressure,
                            crate::workers::handle_connection_httpa!(
                                rth,
                                callback_wrapper,
                                tokio::spawn,
                                hyper_util::rt::TokioExecutor::new,
                                serve_connection_with_upgrades,
                                hyper_util::rt::TokioIo::new,
                                "http",
                                http1_opts,
                                http2_opts,
                                $target
                            )
                        );
                    }
                    ("auto", false) => {
                        crate::workers::handle_connection_loop!(
                            tcp_listener,
                            pyrx.changed(),
                            backpressure,
                            crate::workers::handle_connection_httpa!(
                                rth,
                                callback_wrapper,
                                tokio::spawn,
                                hyper_util::rt::TokioExecutor::new,
                                serve_connection,
                                hyper_util::rt::TokioIo::new,
                                "http",
                                http1_opts,
                                http2_opts,
                                $target
                            )
                        );
                    }
                    ("1", true) => {
                        crate::workers::handle_connection_loop!(
                            tcp_listener,
                            pyrx.changed(),
                            backpressure,
                            crate::workers::handle_connection_http1_upgrades!(
                                rth,
                                callback_wrapper,
                                tokio::spawn,
                                hyper_util::rt::TokioIo::new,
                                "http",
                                http1_opts,
                                $target
                            )
                        );
                    }
                    ("1", false) => {
                        crate::workers::handle_connection_loop!(
                            tcp_listener,
                            pyrx.changed(),
                            backpressure,
                            crate::workers::handle_connection_http1!(
                                rth,
                                callback_wrapper,
                                tokio::spawn,
                                hyper_util::rt::TokioIo::new,
                                "http",
                                http1_opts,
                                $target
                            )
                        );
                    }
                    ("2", _) => {
                        crate::workers::handle_connection_loop!(
                            tcp_listener,
                            pyrx.changed(),
                            backpressure,
                            crate::workers::handle_connection_http2!(
                                rth,
                                callback_wrapper,
                                tokio::spawn,
                                hyper_util::rt::TokioExecutor::new,
                                hyper_util::rt::TokioIo::new,
                                "http",
                                http2_opts,
                                $target
                            )
                        );
                    }
                    _ => unreachable!(),
                }

                Python::with_gil(|_| drop(callback_wrapper));

                log::info!("Stopping worker-{}", worker_id);
                Ok(())
            });

            match svc_loop {
                Ok(()) => {}
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
            event_loop: &Bound<PyAny>,
            context: Bound<PyAny>,
            signal: Py<crate::workers::WorkerSignal>,
        ) {
            pyo3_log::init();
            let rt = crate::runtime::init_runtime_mt(
                self.config.threads,
                self.config.blocking_threads,
                std::sync::Arc::new(event_loop.clone().unbind()),
            );
            let rth = rt.handler();
            let tcp_listener = self.config.tcp_listener();

            let http_mode = self.config.http_mode.clone();
            let http_upgrades = self.config.websockets_enabled;
            let http1_opts = self.config.http1_opts.clone();
            let http2_opts = self.config.http2_opts.clone();
            let backpressure = self.config.backpressure.clone();
            let tls_cfg = self.config.tls_cfg();
            let callback_wrapper = crate::callbacks::CallbackWrapper::new(callback, event_loop.clone(), context);
            let mut pyrx = signal.get().rx.lock().unwrap().take().unwrap();

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let svc_loop = crate::runtime::run_until_complete(rt.handler(), event_loop.clone(), async move {
                match (&http_mode[..], http_upgrades) {
                    ("auto", true) => {
                        crate::workers::handle_tls_loop!(
                            tcp_listener,
                            tls_cfg,
                            pyrx.changed(),
                            backpressure,
                            crate::workers::handle_connection_httpa!(
                                rth,
                                callback_wrapper,
                                tokio::spawn,
                                hyper_util::rt::TokioExecutor::new,
                                serve_connection_with_upgrades,
                                hyper_util::rt::TokioIo::new,
                                "https",
                                http1_opts,
                                http2_opts,
                                $target
                            )
                        );
                    }
                    ("auto", false) => {
                        crate::workers::handle_tls_loop!(
                            tcp_listener,
                            tls_cfg,
                            pyrx.changed(),
                            backpressure,
                            crate::workers::handle_connection_httpa!(
                                rth,
                                callback_wrapper,
                                tokio::spawn,
                                hyper_util::rt::TokioExecutor::new,
                                serve_connection,
                                hyper_util::rt::TokioIo::new,
                                "https",
                                http1_opts,
                                http2_opts,
                                $target
                            )
                        );
                    }
                    ("1", true) => {
                        crate::workers::handle_tls_loop!(
                            tcp_listener,
                            tls_cfg,
                            pyrx.changed(),
                            backpressure,
                            crate::workers::handle_connection_http1_upgrades!(
                                rth,
                                callback_wrapper,
                                tokio::spawn,
                                hyper_util::rt::TokioIo::new,
                                "https",
                                http1_opts,
                                $target
                            )
                        );
                    }
                    ("1", false) => {
                        crate::workers::handle_tls_loop!(
                            tcp_listener,
                            tls_cfg,
                            pyrx.changed(),
                            backpressure,
                            crate::workers::handle_connection_http1!(
                                rth,
                                callback_wrapper,
                                tokio::spawn,
                                hyper_util::rt::TokioIo::new,
                                "https",
                                http1_opts,
                                $target
                            )
                        );
                    }
                    ("2", _) => {
                        crate::workers::handle_tls_loop!(
                            tcp_listener,
                            tls_cfg,
                            pyrx.changed(),
                            backpressure,
                            crate::workers::handle_connection_http2!(
                                rth,
                                callback_wrapper,
                                tokio::spawn,
                                hyper_util::rt::TokioExecutor::new,
                                hyper_util::rt::TokioIo::new,
                                "https",
                                http2_opts,
                                $target
                            )
                        );
                    }
                    _ => unreachable!(),
                }

                Python::with_gil(|_| drop(callback_wrapper));

                log::info!("Stopping worker-{}", worker_id);
                Ok(())
            });

            match svc_loop {
                Ok(()) => {}
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
            event_loop: &Bound<PyAny>,
            context: Bound<PyAny>,
            signal: Py<crate::workers::WorkerSignal>,
        ) {
            pyo3_log::init();
            let rtm = crate::runtime::init_runtime_mt(1, 1, std::sync::Arc::new(event_loop.clone().unbind()));

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let callback_wrapper = crate::callbacks::CallbackWrapper::new(callback, event_loop.clone(), context);
            let py_loop = std::sync::Arc::new(event_loop.clone().unbind());
            let mut pyrx = signal.get().rx.lock().unwrap().take().unwrap();
            let (stx, srx) = tokio::sync::watch::channel(false);
            let mut workers = vec![];

            for thread_id in 0..self.config.threads {
                log::info!("Started worker-{} runtime-{}", worker_id, thread_id + 1);

                let tcp_listener = self.config.tcp_listener();
                let http_mode = self.config.http_mode.clone();
                let http_upgrades = self.config.websockets_enabled;
                let http1_opts = self.config.http1_opts.clone();
                let http2_opts = self.config.http2_opts.clone();
                let blocking_threads = self.config.blocking_threads.clone();
                let backpressure = self.config.backpressure.clone();
                let callback_wrapper = callback_wrapper.clone();
                let py_loop = py_loop.clone();
                let mut srx = srx.clone();

                workers.push(std::thread::spawn(move || {
                    let rt = crate::runtime::init_runtime_st(blocking_threads, py_loop);
                    let rth = rt.handler();
                    let local = tokio::task::LocalSet::new();

                    crate::runtime::block_on_local(rt, local, async move {
                        match (&http_mode[..], http_upgrades) {
                            ("auto", true) => {
                                crate::workers::handle_connection_loop!(
                                    tcp_listener,
                                    srx.changed(),
                                    backpressure,
                                    crate::workers::handle_connection_httpa!(
                                        rth,
                                        callback_wrapper,
                                        tokio::task::spawn_local,
                                        crate::workers::WorkerExecutor::new,
                                        serve_connection_with_upgrades,
                                        hyper_util::rt::TokioIo::new,
                                        "http",
                                        http1_opts,
                                        http2_opts,
                                        $target
                                    )
                                );
                            }
                            ("auto", false) => {
                                crate::workers::handle_connection_loop!(
                                    tcp_listener,
                                    srx.changed(),
                                    backpressure,
                                    crate::workers::handle_connection_httpa!(
                                        rth,
                                        callback_wrapper,
                                        tokio::task::spawn_local,
                                        crate::workers::WorkerExecutor::new,
                                        serve_connection,
                                        hyper_util::rt::TokioIo::new,
                                        "http",
                                        http1_opts,
                                        http2_opts,
                                        $target
                                    )
                                );
                            }
                            ("1", true) => {
                                crate::workers::handle_connection_loop!(
                                    tcp_listener,
                                    srx.changed(),
                                    backpressure,
                                    crate::workers::handle_connection_http1_upgrades!(
                                        rth,
                                        callback_wrapper,
                                        tokio::task::spawn_local,
                                        hyper_util::rt::TokioIo::new,
                                        "http",
                                        http1_opts,
                                        $target
                                    )
                                );
                            }
                            ("1", false) => {
                                crate::workers::handle_connection_loop!(
                                    tcp_listener,
                                    srx.changed(),
                                    backpressure,
                                    crate::workers::handle_connection_http1!(
                                        rth,
                                        callback_wrapper,
                                        tokio::task::spawn_local,
                                        hyper_util::rt::TokioIo::new,
                                        "http",
                                        http1_opts,
                                        $target
                                    )
                                );
                            }
                            ("2", _) => {
                                crate::workers::handle_connection_loop!(
                                    tcp_listener,
                                    srx.changed(),
                                    backpressure,
                                    crate::workers::handle_connection_http2!(
                                        rth,
                                        callback_wrapper,
                                        tokio::task::spawn_local,
                                        crate::workers::WorkerExecutor::new,
                                        |stream| {
                                            crate::io::IOTypeNotSend::new(hyper_util::rt::TokioIo::new(stream))
                                        },
                                        "http",
                                        http2_opts,
                                        $target
                                    )
                                );
                            }
                            _ => unreachable!(),
                        }

                        log::info!("Stopping worker-{} runtime-{}", worker_id, thread_id + 1);
                    });
                }));
            }

            let main_loop = crate::runtime::run_until_complete(rtm.handler(), event_loop.clone(), async move {
                let _ = pyrx.changed().await;
                stx.send(true).unwrap();
                log::info!("Stopping worker-{}", worker_id);
                while let Some(worker) = workers.pop() {
                    worker.join().unwrap();
                }
                Ok(())
            });

            match main_loop {
                Ok(()) => {}
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
            event_loop: &Bound<PyAny>,
            context: Bound<PyAny>,
            signal: Py<crate::workers::WorkerSignal>,
        ) {
            pyo3_log::init();
            let rtm = crate::runtime::init_runtime_mt(1, 1, std::sync::Arc::new(event_loop.clone().unbind()));

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let callback_wrapper = crate::callbacks::CallbackWrapper::new(callback, event_loop.clone(), context);
            let py_loop = std::sync::Arc::new(event_loop.clone().unbind());
            let mut pyrx = signal.get().rx.lock().unwrap().take().unwrap();
            let (stx, srx) = tokio::sync::watch::channel(false);
            let mut workers = vec![];

            for thread_id in 0..self.config.threads {
                log::info!("Started worker-{} runtime-{}", worker_id, thread_id + 1);

                let tcp_listener = self.config.tcp_listener();
                let http_mode = self.config.http_mode.clone();
                let http_upgrades = self.config.websockets_enabled;
                let http1_opts = self.config.http1_opts.clone();
                let http2_opts = self.config.http2_opts.clone();
                let tls_cfg = self.config.tls_cfg();
                let blocking_threads = self.config.blocking_threads.clone();
                let backpressure = self.config.backpressure.clone();
                let callback_wrapper = callback_wrapper.clone();
                let py_loop = py_loop.clone();
                let mut srx = srx.clone();

                workers.push(std::thread::spawn(move || {
                    let rt = crate::runtime::init_runtime_st(blocking_threads, py_loop);
                    let rth = rt.handler();
                    let local = tokio::task::LocalSet::new();

                    crate::runtime::block_on_local(rt, local, async move {
                        match (&http_mode[..], http_upgrades) {
                            ("auto", true) => {
                                crate::workers::handle_tls_loop!(
                                    tcp_listener,
                                    tls_cfg,
                                    srx.changed(),
                                    backpressure,
                                    crate::workers::handle_connection_httpa!(
                                        rth,
                                        callback_wrapper,
                                        tokio::task::spawn_local,
                                        crate::workers::WorkerExecutor::new,
                                        serve_connection_with_upgrades,
                                        hyper_util::rt::TokioIo::new,
                                        "https",
                                        http1_opts,
                                        http2_opts,
                                        $target
                                    )
                                );
                            }
                            ("auto", false) => {
                                crate::workers::handle_tls_loop!(
                                    tcp_listener,
                                    tls_cfg,
                                    srx.changed(),
                                    backpressure,
                                    crate::workers::handle_connection_httpa!(
                                        rth,
                                        callback_wrapper,
                                        tokio::task::spawn_local,
                                        crate::workers::WorkerExecutor::new,
                                        serve_connection,
                                        hyper_util::rt::TokioIo::new,
                                        "https",
                                        http1_opts,
                                        http2_opts,
                                        $target
                                    )
                                );
                            }
                            ("1", true) => {
                                crate::workers::handle_tls_loop!(
                                    tcp_listener,
                                    tls_cfg,
                                    srx.changed(),
                                    backpressure,
                                    crate::workers::handle_connection_http1_upgrades!(
                                        rth,
                                        callback_wrapper,
                                        tokio::task::spawn_local,
                                        hyper_util::rt::TokioIo::new,
                                        "https",
                                        http1_opts,
                                        $target
                                    )
                                );
                            }
                            ("1", false) => {
                                crate::workers::handle_tls_loop!(
                                    tcp_listener,
                                    tls_cfg,
                                    srx.changed(),
                                    backpressure,
                                    crate::workers::handle_connection_http1!(
                                        rth,
                                        callback_wrapper,
                                        tokio::task::spawn_local,
                                        hyper_util::rt::TokioIo::new,
                                        "https",
                                        http1_opts,
                                        $target
                                    )
                                );
                            }
                            ("2", _) => {
                                crate::workers::handle_tls_loop!(
                                    tcp_listener,
                                    tls_cfg,
                                    srx.changed(),
                                    backpressure,
                                    crate::workers::handle_connection_http2!(
                                        rth,
                                        callback_wrapper,
                                        tokio::task::spawn_local,
                                        crate::workers::WorkerExecutor::new,
                                        |stream| {
                                            crate::io::IOTypeNotSend::new(hyper_util::rt::TokioIo::new(stream))
                                        },
                                        "https",
                                        http2_opts,
                                        $target
                                    )
                                );
                            }
                            _ => unreachable!(),
                        }

                        log::info!("Stopping worker-{} runtime-{}", worker_id, thread_id + 1);
                    });
                }));
            }

            let main_loop = crate::runtime::run_until_complete(rtm.handler(), event_loop.clone(), async move {
                let _ = pyrx.changed().await;
                stx.send(true).unwrap();
                log::info!("Stopping worker-{}", worker_id);
                while let Some(worker) = workers.pop() {
                    worker.join().unwrap();
                }
                Ok(())
            });

            match main_loop {
                Ok(()) => {}
                Err(err) => {
                    log::error!("{}", err);
                    std::process::exit(1);
                }
            };
        }
    };
}

pub(crate) use build_service;
pub(crate) use handle_connection_http1;
pub(crate) use handle_connection_http1_upgrades;
pub(crate) use handle_connection_http2;
pub(crate) use handle_connection_httpa;
pub(crate) use handle_connection_loop;
pub(crate) use handle_tls_loop;
pub(crate) use serve_rth;
pub(crate) use serve_rth_ssl;
pub(crate) use serve_wth;
pub(crate) use serve_wth_ssl;

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<WorkerSignal>()?;
    module.add_class::<ASGIWorker>()?;
    module.add_class::<RSGIWorker>()?;
    module.add_class::<WSGIWorker>()?;

    Ok(())
}
