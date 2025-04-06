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

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct WorkerSignalSync {
    pub rx: Mutex<Option<crossbeam_channel::Receiver<bool>>>,
    tx: crossbeam_channel::Sender<bool>,
    #[pyo3(get)]
    pub qs: PyObject,
}

impl WorkerSignalSync {
    pub fn release(&self, py: Python) -> PyResult<PyObject> {
        self.qs.call_method0(py, "set")
    }
}

#[pymethods]
impl WorkerSignalSync {
    #[new]
    fn new(qs: PyObject) -> Self {
        let (tx, rx) = crossbeam_channel::bounded(1);
        Self {
            rx: Mutex::new(Some(rx)),
            tx,
            qs,
        }
    }

    fn set(&self) {
        let _ = self.tx.send(true);
    }
}

#[derive(Clone)]
pub(crate) struct HTTP1Config {
    pub header_read_timeout: core::time::Duration,
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
    sock: (Py<crate::tcp::ListenerSpec>, Option<i32>),
    pub threads: usize,
    pub blocking_threads: usize,
    pub py_threads: usize,
    pub py_threads_idle_timeout: u64,
    pub backpressure: usize,
    pub http_mode: String,
    pub http1_opts: HTTP1Config,
    pub http2_opts: HTTP2Config,
    pub websockets_enabled: bool,
    pub ssl_enabled: bool,
    ssl_cert: Option<String>,
    ssl_key: Option<String>,
    ssl_key_password: Option<String>,
}

impl WorkerConfig {
    pub fn new(
        id: i32,
        sock: (Py<crate::tcp::ListenerSpec>, Option<i32>),
        threads: usize,
        blocking_threads: usize,
        py_threads: usize,
        py_threads_idle_timeout: u64,
        backpressure: usize,
        http_mode: &str,
        http1_opts: HTTP1Config,
        http2_opts: HTTP2Config,
        websockets_enabled: bool,
        ssl_enabled: bool,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
        ssl_key_password: Option<&str>,
    ) -> Self {
        Self {
            id,
            sock,
            threads,
            blocking_threads,
            py_threads,
            py_threads_idle_timeout,
            backpressure,
            http_mode: http_mode.into(),
            http1_opts,
            http2_opts,
            websockets_enabled,
            ssl_enabled,
            ssl_cert: ssl_cert.map(std::convert::Into::into),
            ssl_key: ssl_key.map(std::convert::Into::into),
            ssl_key_password: ssl_key_password.map(std::convert::Into::into),
        }
    }

    #[cfg(unix)]
    pub fn tcp_listener(&self) -> TcpListener {
        let listener = if let Some(fd) = self.sock.1 {
            unsafe { TcpListener::from_raw_fd(fd) }
        } else {
            self.sock.0.get().as_listener().unwrap()
        };
        _ = listener.set_nonblocking(true);
        listener
    }

    #[cfg(windows)]
    pub fn tcp_listener(&self) -> TcpListener {
        let listener = unsafe { TcpListener::from_raw_socket(self.sock.1.unwrap() as u64) };
        _ = listener.set_nonblocking(true);
        listener
    }

    pub fn tls_cfg(&self) -> tls_listener::rustls::rustls::ServerConfig {
        let mut cfg = tls_listener::rustls::rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                tls_load_certs(self.ssl_cert.clone().unwrap()).unwrap(),
                tls_load_pkey(self.ssl_key.clone().unwrap(), self.ssl_key_password.clone()).unwrap(),
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
    ($local_addr:expr, $remote_addr:expr, $callback_wrapper:expr, $rt:expr, $disconnect_guard:expr, $target:expr, $proto:expr) => {
        hyper::service::service_fn(move |request: crate::http::HTTPRequest| {
            let callback_wrapper = $callback_wrapper.clone();
            let rth = $rt.clone();
            let disconnect_guard = $disconnect_guard.clone();

            async move {
                Ok::<_, anyhow::Error>(
                    $target(
                        rth,
                        disconnect_guard,
                        callback_wrapper,
                        $local_addr,
                        $remote_addr,
                        request,
                        $proto,
                    )
                    .await,
                )
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

macro_rules! handle_connection_loop_tls {
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
                let disconnect_guard = std::sync::Arc::new(tokio::sync::Notify::new());
                let disconnect_tx = disconnect_guard.clone();
                let svc = crate::workers::build_service!(
                    local_addr,
                    remote_addr,
                    callback_wrapper,
                    rth,
                    disconnect_guard,
                    $target,
                    $proto
                );
                _ = hyper::server::conn::http1::Builder::new()
                    .timer(crate::io::TokioTimer::new())
                    .header_read_timeout($http_opts.header_read_timeout)
                    .keep_alive($http_opts.keep_alive)
                    .max_buf_size($http_opts.max_buffer_size)
                    .pipeline_flush($http_opts.pipeline_flush)
                    .serve_connection($stream_wrapper(stream), svc)
                    .await;
                disconnect_tx.notify_one();
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
                let disconnect_guard = std::sync::Arc::new(tokio::sync::Notify::new());
                let disconnect_tx = disconnect_guard.clone();
                let svc = crate::workers::build_service!(
                    local_addr,
                    remote_addr,
                    callback_wrapper,
                    rth,
                    disconnect_guard,
                    $target,
                    $proto
                );
                _ = hyper::server::conn::http1::Builder::new()
                    .timer(crate::io::TokioTimer::new())
                    .header_read_timeout($http_opts.header_read_timeout)
                    .keep_alive($http_opts.keep_alive)
                    .max_buf_size($http_opts.max_buffer_size)
                    .pipeline_flush($http_opts.pipeline_flush)
                    .serve_connection($stream_wrapper(stream), svc)
                    .with_upgrades()
                    .await;
                disconnect_tx.notify_one();
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
                let disconnect_guard = std::sync::Arc::new(tokio::sync::Notify::new());
                let disconnect_tx = disconnect_guard.clone();
                let svc = crate::workers::build_service!(
                    local_addr,
                    remote_addr,
                    callback_wrapper,
                    rth,
                    disconnect_guard,
                    $target,
                    $proto
                );
                _ = hyper::server::conn::http2::Builder::new($executor_builder())
                    .timer(crate::io::TokioTimer::new())
                    .adaptive_window($http_opts.adaptive_window)
                    .initial_connection_window_size($http_opts.initial_connection_window_size)
                    .initial_stream_window_size($http_opts.initial_stream_window_size)
                    .keep_alive_interval($http_opts.keep_alive_interval)
                    .keep_alive_timeout($http_opts.keep_alive_timeout)
                    .max_concurrent_streams($http_opts.max_concurrent_streams)
                    .max_frame_size($http_opts.max_frame_size)
                    .max_header_list_size($http_opts.max_headers_size)
                    .max_send_buf_size($http_opts.max_send_buffer_size)
                    .serve_connection($stream_wrapper(stream), svc)
                    .await;
                disconnect_tx.notify_one();
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
                let disconnect_guard = std::sync::Arc::new(tokio::sync::Notify::new());
                let disconnect_tx = disconnect_guard.clone();
                let svc = crate::workers::build_service!(
                    local_addr,
                    remote_addr,
                    callback_wrapper,
                    rth,
                    disconnect_guard,
                    $target,
                    $proto
                );
                let mut conn = hyper_util::server::conn::auto::Builder::new($executor_builder());
                conn.http1()
                    .timer(crate::io::TokioTimer::new())
                    .header_read_timeout($http1_opts.header_read_timeout)
                    .keep_alive($http1_opts.keep_alive)
                    .max_buf_size($http1_opts.max_buffer_size)
                    .pipeline_flush($http1_opts.pipeline_flush);
                conn.http2()
                    .timer(crate::io::TokioTimer::new())
                    .adaptive_window($http2_opts.adaptive_window)
                    .initial_connection_window_size($http2_opts.initial_connection_window_size)
                    .initial_stream_window_size($http2_opts.initial_stream_window_size)
                    .keep_alive_interval($http2_opts.keep_alive_interval)
                    .keep_alive_timeout($http2_opts.keep_alive_timeout)
                    .max_concurrent_streams($http2_opts.max_concurrent_streams)
                    .max_frame_size($http2_opts.max_frame_size)
                    .max_header_list_size($http2_opts.max_headers_size)
                    .max_send_buf_size($http2_opts.max_send_buffer_size);
                _ = conn.$conn_method($stream_wrapper(stream), svc).await;
                disconnect_tx.notify_one();
                drop(permit);
            });
        }
    };
}

macro_rules! loop_match {
    (
        $http_mode:expr,
        $http_upgrades:expr,
        $tcp_listener:expr,
        $pyrx:expr,
        $backpressure:expr,
        $rth:expr,
        $callback_wrapper:expr,
        $spawner:expr,
        $executor:expr,
        $http1_opts:expr,
        $http2_opts:expr,
        $http2_stream_wrapper:expr,
        $target:expr
    ) => {
        match (&$http_mode[..], $http_upgrades) {
            ("auto", true) => {
                crate::workers::handle_connection_loop!(
                    $tcp_listener,
                    $pyrx.changed(),
                    $backpressure,
                    crate::workers::handle_connection_httpa!(
                        $rth,
                        $callback_wrapper,
                        $spawner,
                        $executor,
                        serve_connection_with_upgrades,
                        hyper_util::rt::TokioIo::new,
                        "http",
                        $http1_opts,
                        $http2_opts,
                        $target
                    )
                );
            }
            ("auto", false) => {
                crate::workers::handle_connection_loop!(
                    $tcp_listener,
                    $pyrx.changed(),
                    $backpressure,
                    crate::workers::handle_connection_httpa!(
                        $rth,
                        $callback_wrapper,
                        $spawner,
                        $executor,
                        serve_connection,
                        hyper_util::rt::TokioIo::new,
                        "http",
                        $http1_opts,
                        $http2_opts,
                        $target
                    )
                );
            }
            ("1", true) => {
                crate::workers::handle_connection_loop!(
                    $tcp_listener,
                    $pyrx.changed(),
                    $backpressure,
                    crate::workers::handle_connection_http1_upgrades!(
                        $rth,
                        $callback_wrapper,
                        $spawner,
                        hyper_util::rt::TokioIo::new,
                        "http",
                        $http1_opts,
                        $target
                    )
                );
            }
            ("1", false) => {
                crate::workers::handle_connection_loop!(
                    $tcp_listener,
                    $pyrx.changed(),
                    $backpressure,
                    crate::workers::handle_connection_http1!(
                        $rth,
                        $callback_wrapper,
                        $spawner,
                        hyper_util::rt::TokioIo::new,
                        "http",
                        $http1_opts,
                        $target
                    )
                );
            }
            ("2", _) => {
                crate::workers::handle_connection_loop!(
                    $tcp_listener,
                    $pyrx.changed(),
                    $backpressure,
                    crate::workers::handle_connection_http2!(
                        $rth,
                        $callback_wrapper,
                        $spawner,
                        $executor,
                        $http2_stream_wrapper,
                        "http",
                        $http2_opts,
                        $target
                    )
                );
            }
            _ => unreachable!(),
        }
    };
}

macro_rules! loop_match_tls {
    (
        $http_mode:expr,
        $http_upgrades:expr,
        $tcp_listener:expr,
        $tls_config:expr,
        $pyrx:expr,
        $backpressure:expr,
        $rth:expr,
        $callback_wrapper:expr,
        $spawner:expr,
        $executor:expr,
        $http1_opts:expr,
        $http2_opts:expr,
        $http2_stream_wrapper:expr,
        $target:expr
    ) => {
        match (&$http_mode[..], $http_upgrades) {
            ("auto", true) => {
                crate::workers::handle_connection_loop_tls!(
                    $tcp_listener,
                    $tls_config,
                    $pyrx.changed(),
                    $backpressure,
                    crate::workers::handle_connection_httpa!(
                        $rth,
                        $callback_wrapper,
                        $spawner,
                        $executor,
                        serve_connection_with_upgrades,
                        hyper_util::rt::TokioIo::new,
                        "https",
                        $http1_opts,
                        $http2_opts,
                        $target
                    )
                );
            }
            ("auto", false) => {
                crate::workers::handle_connection_loop_tls!(
                    $tcp_listener,
                    $tls_config,
                    $pyrx.changed(),
                    $backpressure,
                    crate::workers::handle_connection_httpa!(
                        $rth,
                        $callback_wrapper,
                        $spawner,
                        $executor,
                        serve_connection,
                        hyper_util::rt::TokioIo::new,
                        "https",
                        $http1_opts,
                        $http2_opts,
                        $target
                    )
                );
            }
            ("1", true) => {
                crate::workers::handle_connection_loop_tls!(
                    $tcp_listener,
                    $tls_config,
                    $pyrx.changed(),
                    $backpressure,
                    crate::workers::handle_connection_http1_upgrades!(
                        $rth,
                        $callback_wrapper,
                        $spawner,
                        hyper_util::rt::TokioIo::new,
                        "https",
                        $http1_opts,
                        $target
                    )
                );
            }
            ("1", false) => {
                crate::workers::handle_connection_loop_tls!(
                    $tcp_listener,
                    $tls_config,
                    $pyrx.changed(),
                    $backpressure,
                    crate::workers::handle_connection_http1!(
                        $rth,
                        $callback_wrapper,
                        $spawner,
                        hyper_util::rt::TokioIo::new,
                        "https",
                        $http1_opts,
                        $target
                    )
                );
            }
            ("2", _) => {
                crate::workers::handle_connection_loop_tls!(
                    $tcp_listener,
                    $tls_config,
                    $pyrx.changed(),
                    $backpressure,
                    crate::workers::handle_connection_http2!(
                        $rth,
                        $callback_wrapper,
                        $spawner,
                        $executor,
                        $http2_stream_wrapper,
                        "https",
                        $http2_opts,
                        $target
                    )
                );
            }
            _ => unreachable!(),
        }
    };
}

macro_rules! serve_mtr {
    ($func_name:ident, $target:expr) => {
        fn $func_name(
            &self,
            py: Python,
            callback: Py<crate::callbacks::CallbackScheduler>,
            event_loop: &Bound<PyAny>,
            signal: Py<WorkerSignal>,
        ) {
            _ = pyo3_log::try_init();

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let tcp_listener = self.config.tcp_listener();
            let http_mode = self.config.http_mode.clone();
            let http_upgrades = self.config.websockets_enabled;
            let http1_opts = self.config.http1_opts.clone();
            let http2_opts = self.config.http2_opts.clone();
            let backpressure = self.config.backpressure.clone();
            let callback_wrapper = std::sync::Arc::new(callback);
            let rtpyloop = std::sync::Arc::new(event_loop.clone().unbind());

            let rt = py.allow_threads(|| {
                let ret = crate::runtime::init_runtime_mt(
                    self.config.threads,
                    self.config.blocking_threads,
                    self.config.py_threads,
                    self.config.py_threads_idle_timeout,
                    rtpyloop,
                );
                ret
            });
            let rth = rt.handler();
            let tasks = tokio_util::task::TaskTracker::new();
            let mut srx = signal.get().rx.lock().unwrap().take().unwrap();

            let main_loop = crate::runtime::run_until_complete(rt, event_loop.clone(), async move {
                crate::workers::loop_match!(
                    http_mode,
                    http_upgrades,
                    tcp_listener,
                    srx,
                    backpressure,
                    rth,
                    callback_wrapper,
                    |task| tasks.spawn(task),
                    hyper_util::rt::TokioExecutor::new,
                    http1_opts,
                    http2_opts,
                    hyper_util::rt::TokioIo::new,
                    $target
                );

                log::info!("Stopping worker-{}", worker_id);

                tasks.close();
                tasks.wait().await;

                Python::with_gil(|_| drop(callback_wrapper));
                Ok(())
            });

            if let Err(err) = main_loop {
                log::error!("{}", err);
                std::process::exit(1);
            }
        }
    };
}

macro_rules! serve_mtr_ssl {
    ($func_name:ident, $target:expr) => {
        fn $func_name(
            &self,
            py: Python,
            callback: Py<crate::callbacks::CallbackScheduler>,
            event_loop: &Bound<PyAny>,
            signal: Py<WorkerSignal>,
        ) {
            _ = pyo3_log::try_init();

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let tcp_listener = self.config.tcp_listener();
            let http_mode = self.config.http_mode.clone();
            let http_upgrades = self.config.websockets_enabled;
            let http1_opts = self.config.http1_opts.clone();
            let http2_opts = self.config.http2_opts.clone();
            let backpressure = self.config.backpressure.clone();
            let tls_cfg = self.config.tls_cfg();
            let callback_wrapper = std::sync::Arc::new(callback);
            let rtpyloop = std::sync::Arc::new(event_loop.clone().unbind());

            let rt = py.allow_threads(|| {
                let ret = crate::runtime::init_runtime_mt(
                    self.config.threads,
                    self.config.blocking_threads,
                    self.config.py_threads,
                    self.config.py_threads_idle_timeout,
                    rtpyloop,
                );
                ret
            });
            let rth = rt.handler();
            let tasks = tokio_util::task::TaskTracker::new();
            let mut srx = signal.get().rx.lock().unwrap().take().unwrap();

            let main_loop = crate::runtime::run_until_complete(rt, event_loop.clone(), async move {
                crate::workers::loop_match_tls!(
                    http_mode,
                    http_upgrades,
                    tcp_listener,
                    tls_cfg,
                    srx,
                    backpressure,
                    rth,
                    callback_wrapper,
                    |task| tasks.spawn(task),
                    hyper_util::rt::TokioExecutor::new,
                    http1_opts,
                    http2_opts,
                    hyper_util::rt::TokioIo::new,
                    $target
                );

                log::info!("Stopping worker-{}", worker_id);

                tasks.close();
                tasks.wait().await;

                Python::with_gil(|_| drop(callback_wrapper));
                Ok(())
            });

            if let Err(err) = main_loop {
                log::error!("{}", err);
                std::process::exit(1);
            }
        }
    };
}

macro_rules! serve_str_inner {
    ($self:expr, $target:expr, $callback:expr, $event_loop:expr, $wid:expr, $workers:expr, $srx:expr) => {
        let callback_wrapper = std::sync::Arc::new($callback);
        let py_loop = std::sync::Arc::new($event_loop.clone().unbind());

        for thread_id in 0..$self.config.threads {
            log::info!("Started worker-{} runtime-{}", $wid, thread_id + 1);

            let tcp_listener = $self.config.tcp_listener();
            let http_mode = $self.config.http_mode.clone();
            let http_upgrades = $self.config.websockets_enabled;
            let http1_opts = $self.config.http1_opts.clone();
            let http2_opts = $self.config.http2_opts.clone();
            let blocking_threads = $self.config.blocking_threads.clone();
            let py_threads = $self.config.py_threads.clone();
            let py_threads_idle_timeout = $self.config.py_threads_idle_timeout.clone();
            let backpressure = $self.config.backpressure.clone();
            let callback_wrapper = callback_wrapper.clone();
            let py_loop = py_loop.clone();
            let mut srx = $srx.clone();

            $workers.push(std::thread::spawn(move || {
                let rt =
                    crate::runtime::init_runtime_st(blocking_threads, py_threads, py_threads_idle_timeout, py_loop);
                let rth = rt.handler();
                let local = tokio::task::LocalSet::new();
                let tasks = tokio_util::task::TaskTracker::new();

                crate::runtime::block_on_local(&rt, local, async move {
                    crate::workers::loop_match!(
                        http_mode,
                        http_upgrades,
                        tcp_listener,
                        srx,
                        backpressure,
                        rth,
                        callback_wrapper,
                        |task| tasks.spawn_local(task),
                        crate::workers::WorkerExecutor::new,
                        http1_opts,
                        http2_opts,
                        |stream| { crate::io::IOTypeNotSend::new(hyper_util::rt::TokioIo::new(stream)) },
                        $target
                    );

                    log::info!("Stopping worker-{} runtime-{}", $wid, thread_id + 1);

                    tasks.close();
                    tasks.wait().await;

                    Python::with_gil(|_| drop(callback_wrapper));
                });

                Python::with_gil(|_| drop(rt));
            }));
        }
    };
}

macro_rules! serve_str {
    ($func_name:ident, $target:expr) => {
        fn $func_name(
            &self,
            callback: Py<crate::callbacks::CallbackScheduler>,
            event_loop: &Bound<PyAny>,
            signal: Py<WorkerSignal>,
        ) {
            _ = pyo3_log::try_init();

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let (stx, srx) = tokio::sync::watch::channel(false);
            let mut workers = vec![];
            crate::workers::serve_str_inner!(self, $target, callback, event_loop, worker_id, workers, srx);

            let rtm = crate::runtime::init_runtime_mt(1, 1, 0, 0, std::sync::Arc::new(event_loop.clone().unbind()));
            let mut pyrx = signal.get().rx.lock().unwrap().take().unwrap();
            let main_loop = crate::runtime::run_until_complete(rtm, event_loop.clone(), async move {
                let _ = pyrx.changed().await;
                stx.send(true).unwrap();
                log::info!("Stopping worker-{}", worker_id);
                while let Some(worker) = workers.pop() {
                    worker.join().unwrap();
                }
                Ok(())
            });

            if let Err(err) = main_loop {
                log::error!("{}", err);
                std::process::exit(1);
            }
        }
    };
}

macro_rules! serve_str_ssl_inner {
    ($self:expr, $target:expr, $callback:expr, $event_loop:expr, $wid:expr, $workers:expr, $srx:expr) => {
        let callback_wrapper = std::sync::Arc::new($callback);
        let py_loop = std::sync::Arc::new($event_loop.clone().unbind());

        for thread_id in 0..$self.config.threads {
            log::info!("Started worker-{} runtime-{}", $wid, thread_id + 1);

            let tcp_listener = $self.config.tcp_listener();
            let http_mode = $self.config.http_mode.clone();
            let http_upgrades = $self.config.websockets_enabled;
            let http1_opts = $self.config.http1_opts.clone();
            let http2_opts = $self.config.http2_opts.clone();
            let tls_cfg = $self.config.tls_cfg();
            let blocking_threads = $self.config.blocking_threads.clone();
            let py_threads = $self.config.py_threads.clone();
            let py_threads_idle_timeout = $self.config.py_threads_idle_timeout.clone();
            let backpressure = $self.config.backpressure.clone();
            let callback_wrapper = callback_wrapper.clone();
            let py_loop = py_loop.clone();
            let mut srx = $srx.clone();

            $workers.push(std::thread::spawn(move || {
                let rt =
                    crate::runtime::init_runtime_st(blocking_threads, py_threads, py_threads_idle_timeout, py_loop);
                let rth = rt.handler();
                let local = tokio::task::LocalSet::new();
                let tasks = tokio_util::task::TaskTracker::new();

                crate::runtime::block_on_local(&rt, local, async move {
                    crate::workers::loop_match_tls!(
                        http_mode,
                        http_upgrades,
                        tcp_listener,
                        tls_cfg,
                        srx,
                        backpressure,
                        rth,
                        callback_wrapper,
                        |task| tasks.spawn_local(task),
                        crate::workers::WorkerExecutor::new,
                        http1_opts,
                        http2_opts,
                        |stream| { crate::io::IOTypeNotSend::new(hyper_util::rt::TokioIo::new(stream)) },
                        $target
                    );

                    log::info!("Stopping worker-{} runtime-{}", $wid, thread_id + 1);

                    tasks.close();
                    tasks.wait().await;

                    Python::with_gil(|_| drop(callback_wrapper));
                });

                Python::with_gil(|_| drop(rt));
            }));
        }
    };
}

macro_rules! serve_str_ssl {
    ($func_name:ident, $target:expr) => {
        fn $func_name(
            &self,
            callback: Py<crate::callbacks::CallbackScheduler>,
            event_loop: &Bound<PyAny>,
            signal: Py<WorkerSignal>,
        ) {
            _ = pyo3_log::try_init();

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let (stx, srx) = tokio::sync::watch::channel(false);
            let mut workers = vec![];
            crate::workers::serve_str_ssl_inner!(self, $target, callback, event_loop, worker_id, workers, srx);

            let rtm = crate::runtime::init_runtime_mt(1, 1, 0, 0, std::sync::Arc::new(event_loop.clone().unbind()));
            let mut pyrx = signal.get().rx.lock().unwrap().take().unwrap();
            let main_loop = crate::runtime::run_until_complete(rtm, event_loop.clone(), async move {
                let _ = pyrx.changed().await;
                stx.send(true).unwrap();
                log::info!("Stopping worker-{}", worker_id);
                while let Some(worker) = workers.pop() {
                    worker.join().unwrap();
                }
                Ok(())
            });

            if let Err(err) = main_loop {
                log::error!("{}", err);
                std::process::exit(1);
            }
        }
    };
}

macro_rules! serve_fut {
    ($func_name:ident, $target:expr) => {
        fn $func_name<'p>(
            &self,
            callback: Py<crate::callbacks::CallbackScheduler>,
            event_loop: &Bound<'p, PyAny>,
            signal: Py<WorkerSignal>,
        ) -> Bound<'p, PyAny> {
            _ = pyo3_log::try_init();

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let (stx, srx) = tokio::sync::watch::channel(false);
            let mut workers = vec![];
            crate::workers::serve_str_inner!(self, $target, callback, event_loop, worker_id, workers, srx);

            let ret = event_loop.call_method0("create_future").unwrap();
            let pyfut = ret.clone().unbind();
            let pyloop = event_loop.clone().unbind();

            std::thread::spawn(move || {
                let pyloop = std::sync::Arc::new(pyloop);
                let rt = crate::runtime::init_runtime_st(1, 0, 0, pyloop.clone());
                let local = tokio::task::LocalSet::new();

                let mut pyrx = signal.get().rx.lock().unwrap().take().unwrap();
                crate::runtime::block_on_local(&rt, local, async move {
                    let _ = pyrx.changed().await;
                    stx.send(true).unwrap();
                    log::info!("Stopping worker-{}", worker_id);
                    while let Some(worker) = workers.pop() {
                        worker.join().unwrap();
                    }
                });

                Python::with_gil(|py| {
                    let cb = pyfut.getattr(py, "set_result").unwrap();
                    _ = pyloop.call_method1(
                        py,
                        "call_soon_threadsafe",
                        (crate::callbacks::PyFutureResultSetter, cb, py.None()),
                    );
                    drop(pyfut);
                    drop(pyloop);
                    drop(signal);
                    drop(rt);
                });
            });

            ret
        }
    };
}

macro_rules! serve_fut_ssl {
    ($func_name:ident, $target:expr) => {
        fn $func_name<'p>(
            &self,
            callback: Py<crate::callbacks::CallbackScheduler>,
            event_loop: &Bound<'p, PyAny>,
            signal: Py<WorkerSignal>,
        ) -> Bound<'p, PyAny> {
            _ = pyo3_log::try_init();

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let (stx, srx) = tokio::sync::watch::channel(false);
            let mut workers = vec![];
            crate::workers::serve_str_ssl_inner!(self, $target, callback, event_loop, worker_id, workers, srx);

            let ret = event_loop.call_method0("create_future").unwrap();
            let pyfut = ret.clone().unbind();
            let pyloop = event_loop.clone().unbind();

            std::thread::spawn(move || {
                let pyloop = std::sync::Arc::new(pyloop);
                let rt = crate::runtime::init_runtime_st(1, 0, 0, pyloop.clone());
                let local = tokio::task::LocalSet::new();

                let mut pyrx = signal.get().rx.lock().unwrap().take().unwrap();
                crate::runtime::block_on_local(&rt, local, async move {
                    let _ = pyrx.changed().await;
                    stx.send(true).unwrap();
                    log::info!("Stopping worker-{}", worker_id);
                    while let Some(worker) = workers.pop() {
                        worker.join().unwrap();
                    }
                });

                Python::with_gil(|py| {
                    let cb = pyfut.getattr(py, "set_result").unwrap();
                    _ = pyloop.call_method1(
                        py,
                        "call_soon_threadsafe",
                        (crate::callbacks::PyFutureResultSetter, cb, py.None()),
                    );
                    drop(pyfut);
                    drop(pyloop);
                    drop(signal);
                    drop(rt);
                });
            });

            ret
        }
    };
}

pub(crate) use build_service;
pub(crate) use handle_connection_http1;
pub(crate) use handle_connection_http1_upgrades;
pub(crate) use handle_connection_http2;
pub(crate) use handle_connection_httpa;
pub(crate) use handle_connection_loop;
pub(crate) use handle_connection_loop_tls;
pub(crate) use loop_match;
pub(crate) use loop_match_tls;
pub(crate) use serve_fut;
pub(crate) use serve_fut_ssl;
pub(crate) use serve_mtr;
pub(crate) use serve_mtr_ssl;
pub(crate) use serve_str;
pub(crate) use serve_str_inner;
pub(crate) use serve_str_ssl;
pub(crate) use serve_str_ssl_inner;

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<WorkerSignal>()?;
    module.add_class::<WorkerSignalSync>()?;
    module.add_class::<ASGIWorker>()?;
    module.add_class::<RSGIWorker>()?;
    module.add_class::<WSGIWorker>()?;

    Ok(())
}
