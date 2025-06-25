use pyo3::prelude::*;
use std::net::TcpListener;
use std::sync::Mutex;

use super::asgi::serve::ASGIWorker;
use super::rsgi::serve::RSGIWorker;
use super::tls::{load_certs as tls_load_certs, load_crls as tls_load_crls, load_private_key as tls_load_pkey};
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
    sock: Py<crate::tcp::SocketHolder>,
    pub threads: usize,
    pub blocking_threads: usize,
    pub py_threads: usize,
    pub py_threads_idle_timeout: u64,
    pub backpressure: usize,
    pub http_mode: String,
    pub http1_opts: HTTP1Config,
    pub http2_opts: HTTP2Config,
    pub websockets_enabled: bool,
    pub static_files: Option<(String, String, String)>,
    pub tls_opts: Option<WorkerTlsConfig>,
}

pub(crate) struct WorkerTlsConfig {
    cert: String,
    key: (String, Option<String>),
    ca: Option<String>,
    crl: Vec<String>,
    client_verify: bool,
}

impl WorkerConfig {
    pub fn new(
        id: i32,
        sock: Py<crate::tcp::SocketHolder>,
        threads: usize,
        blocking_threads: usize,
        py_threads: usize,
        py_threads_idle_timeout: u64,
        backpressure: usize,
        http_mode: &str,
        http1_opts: HTTP1Config,
        http2_opts: HTTP2Config,
        websockets_enabled: bool,
        static_files: Option<(String, String, String)>,
        ssl_enabled: bool,
        ssl_cert: Option<String>,
        ssl_key: Option<String>,
        ssl_key_password: Option<String>,
        ssl_ca: Option<String>,
        ssl_crl: Vec<String>,
        ssl_client_verify: bool,
    ) -> Self {
        let tls_opts = match ssl_enabled {
            true => Some(WorkerTlsConfig {
                cert: ssl_cert.unwrap(),
                key: (ssl_key.unwrap(), ssl_key_password),
                ca: ssl_ca,
                crl: ssl_crl,
                client_verify: ssl_client_verify,
            }),
            false => None,
        };

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
            static_files,
            tls_opts,
        }
    }

    pub fn tcp_listener(&self) -> TcpListener {
        let listener = self.sock.get().as_listener().unwrap();
        _ = listener.set_nonblocking(true);
        listener
    }

    pub fn tls_cfg(&self) -> tls_listener::rustls::rustls::ServerConfig {
        let opts = self.tls_opts.as_ref().unwrap();
        let cfg_builder = match &opts.ca {
            Some(ca) => {
                let cas = tls_load_certs(ca.clone());
                let mut client_auth_cas = tls_listener::rustls::rustls::RootCertStore::empty();
                for cert in cas {
                    client_auth_cas.add(cert).unwrap();
                }
                let crls = tls_load_crls(opts.crl.iter());
                let verifier = match opts.client_verify {
                    true => tls_listener::rustls::rustls::server::WebPkiClientVerifier::builder(client_auth_cas.into())
                        .with_crls(crls)
                        .build()
                        .unwrap(),
                    false => {
                        tls_listener::rustls::rustls::server::WebPkiClientVerifier::builder(client_auth_cas.into())
                            .with_crls(crls)
                            .allow_unauthenticated()
                            .build()
                            .unwrap()
                    }
                };
                tls_listener::rustls::rustls::ServerConfig::builder().with_client_cert_verifier(verifier)
            }
            None => tls_listener::rustls::rustls::ServerConfig::builder().with_no_client_auth(),
        };
        let mut cfg = cfg_builder
            .with_single_cert(
                tls_load_certs(opts.cert.clone()),
                tls_load_pkey(opts.key.0.clone(), opts.key.1.clone()),
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

pub(crate) trait WorkerCTX {
    type CTX;

    fn get_ctx(&self) -> std::sync::Arc<Self::CTX>;
}

pub(crate) struct WorkerCTXBase {
    pub callback: crate::callbacks::ArcCBScheduler,
}

impl WorkerCTXBase {
    pub fn new(callback: crate::callbacks::PyCBScheduler, _files: Option<(String, String, String)>) -> Self {
        Self {
            callback: std::sync::Arc::new(callback),
        }
    }
}

pub(crate) struct WorkerCTXFiles {
    pub callback: crate::callbacks::ArcCBScheduler,
    pub static_prefix: String,
    pub static_mount: String,
    pub static_expires: String,
}

impl WorkerCTXFiles {
    pub fn new(callback: crate::callbacks::PyCBScheduler, files: Option<(String, String, String)>) -> Self {
        let (static_prefix, static_mount, static_expires) = files.unwrap();
        Self {
            callback: std::sync::Arc::new(callback),
            static_prefix,
            static_mount,
            static_expires,
        }
    }
}

pub(crate) struct Worker<C> {
    ctx: std::sync::Arc<C>,
}

impl<C> Worker<C> {
    pub fn new(ctx: C) -> Self {
        Self {
            ctx: std::sync::Arc::new(ctx),
        }
    }
}

impl<C> WorkerCTX for Worker<C> {
    type CTX = C;

    fn get_ctx(&self) -> std::sync::Arc<Self::CTX> {
        self.ctx.clone()
    }
}

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

macro_rules! service_app {
    ($target:expr, $rt:expr, $ctx:expr, $disconnect_guard:expr, $local_addr:expr, $remote_addr:expr, $proto:expr, $request:expr) => {{
        let rt = $rt.clone();
        let callback = $ctx.callback.clone();
        let disconnect_guard = $disconnect_guard.clone();

        async move {
            Ok::<_, anyhow::Error>(
                $target(
                    rt,
                    disconnect_guard,
                    callback,
                    $local_addr,
                    $remote_addr,
                    $request,
                    $proto,
                )
                .await,
            )
        }
    }};
}

macro_rules! service_files {
    ($target:expr, $rt:expr, $ctx:expr, $disconnect_guard:expr, $local_addr:expr, $remote_addr:expr, $proto:expr, $request:expr) => {{
        if let Some(static_match) =
            crate::files::match_static_file($request.uri().path(), &$ctx.static_prefix, &$ctx.static_mount)
        {
            if static_match.is_err() {
                return async move { Ok::<_, anyhow::Error>(crate::http::response_404()) }.boxed();
            }
            let expires = $ctx.static_expires.clone();
            return async move {
                Ok::<_, anyhow::Error>(crate::files::serve_static_file(static_match.unwrap(), expires).await)
            }
            .boxed();
        }

        crate::workers::service_app!(
            $target,
            $rt,
            $ctx,
            $disconnect_guard,
            $local_addr,
            $remote_addr,
            $proto,
            $request
        )
        .boxed()
    }};
}

macro_rules! build_service_fn {
    ($builder:ident, $target:expr, $rt:expr, $ctx:expr, $disconnect_guard:expr, $local_addr:expr, $remote_addr:expr, $proto:expr) => {
        hyper::service::service_fn(move |request: crate::http::HTTPRequest| {
            crate::workers::$builder!(
                $target,
                $rt,
                $ctx,
                $disconnect_guard,
                $local_addr,
                $remote_addr,
                $proto,
                request
            )
        })
    };
}

macro_rules! connection_builder_h1 {
    ($opts:expr, $stream:expr, $svc:expr) => {
        hyper::server::conn::http1::Builder::new()
            .timer(crate::io::TokioTimer::new())
            .header_read_timeout($opts.header_read_timeout)
            .keep_alive($opts.keep_alive)
            .max_buf_size($opts.max_buffer_size)
            .pipeline_flush($opts.pipeline_flush)
            .serve_connection($stream, $svc)
    };
}

macro_rules! connection_builder_h1u {
    ($opts:expr, $stream:expr, $svc:expr) => {
        crate::workers::connection_builder_h1!($opts, $stream, $svc).with_upgrades()
    };
}

macro_rules! connection_handler {
    (
        auto
        $ctx:expr,
        $target:expr,
        $svc:ident,
        $rt:expr,
        $spawner:expr,
        $executor:expr,
        $conn_method:ident,
        $http1_opts:expr,
        $http2_opts:expr
    ) => {
        |local_addr, remote_addr, stream, permit, sig: std::sync::Arc<tokio::sync::Notify>, proto| {
            let rt = $rt.clone();
            let ctx = $ctx.clone();

            $spawner(async move {
                let disconnect_guard = std::sync::Arc::new(tokio::sync::Notify::new());
                let disconnect_tx = disconnect_guard.clone();
                let svc = crate::workers::build_service_fn!(
                    $svc,
                    $target,
                    rt,
                    ctx,
                    disconnect_guard,
                    local_addr,
                    remote_addr,
                    proto
                );

                let mut connb = hyper_util::server::conn::auto::Builder::new($executor());
                connb
                    .http1()
                    .timer(crate::io::TokioTimer::new())
                    .header_read_timeout($http1_opts.header_read_timeout)
                    .keep_alive($http1_opts.keep_alive)
                    .max_buf_size($http1_opts.max_buffer_size)
                    .pipeline_flush($http1_opts.pipeline_flush);
                connb
                    .http2()
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

                let mut done = false;
                let conn = connb.$conn_method(hyper_util::rt::TokioIo::new(stream), svc);
                tokio::pin!(conn);
                tokio::select! {
                    _ = conn.as_mut() => {
                        done = true;
                    },
                    _ = sig.notified() => {
                        conn.as_mut().graceful_shutdown();
                    }
                }
                if !done {
                    _ = conn.as_mut().await;
                }

                disconnect_tx.notify_one();
                drop(permit);
            });
        }
    };
    (
        1
        $ctx:expr,
        $target:expr,
        $svc:ident,
        $rt:expr,
        $spawner:expr,
        $conn_method:ident,
        $http_opts:expr
    ) => {
        |local_addr, remote_addr, stream, permit, sig: std::sync::Arc<tokio::sync::Notify>, proto| {
            let rt = $rt.clone();
            let ctx = $ctx.clone();

            $spawner(async move {
                let disconnect_guard = std::sync::Arc::new(tokio::sync::Notify::new());
                let disconnect_tx = disconnect_guard.clone();
                let svc = crate::workers::build_service_fn!(
                    $svc,
                    $target,
                    rt,
                    ctx,
                    disconnect_guard,
                    local_addr,
                    remote_addr,
                    proto
                );

                let mut done = false;
                let conn = crate::workers::$conn_method!($http_opts, hyper_util::rt::TokioIo::new(stream), svc);
                tokio::pin!(conn);
                tokio::select! {
                    _ = conn.as_mut() => {
                        done = true;
                    },
                    _ = sig.notified() => {
                        conn.as_mut().graceful_shutdown();
                    }
                }
                if !done {
                    _ = conn.as_mut().await;
                }

                disconnect_tx.notify_one();
                drop(permit);
            });
        }
    };
    (
        2
        $ctx:expr,
        $target:expr,
        $svc:ident,
        $rt:expr,
        $spawner:expr,
        $executor:expr,
        $stream_wrapper:expr,
        $http_opts:expr
    ) => {
        |local_addr, remote_addr, stream, permit, sig: std::sync::Arc<tokio::sync::Notify>, proto| {
            let rt = $rt.clone();
            let ctx = $ctx.clone();

            $spawner(async move {
                let disconnect_guard = std::sync::Arc::new(tokio::sync::Notify::new());
                let disconnect_tx = disconnect_guard.clone();
                let svc = crate::workers::build_service_fn!(
                    $svc,
                    $target,
                    rt,
                    ctx,
                    disconnect_guard,
                    local_addr,
                    remote_addr,
                    proto
                );

                let mut done = false;
                let conn = hyper::server::conn::http2::Builder::new($executor())
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
                    .serve_connection($stream_wrapper(stream), svc);
                tokio::pin!(conn);
                tokio::select! {
                    _ = conn.as_mut() => {
                        done = true;
                    },
                    _ = sig.notified() => {
                        conn.as_mut().graceful_shutdown();
                    }
                }
                if !done {
                    _ = conn.as_mut().await;
                }

                disconnect_tx.notify_one();
                drop(permit);
            });
        }
    };
}

macro_rules! accept_loop {
    (
        plain
        $listener:expr,
        $listener_cfg:expr,
        $pysig:expr,
        $backpressure:expr,
        $handler:expr
    ) => {{
        let tcp_listener = tokio::net::TcpListener::from_std($listener).unwrap();
        let local_addr = tcp_listener.local_addr().unwrap();
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new($backpressure));
        let connsig = std::sync::Arc::new(tokio::sync::Notify::new());
        let mut accept_loop = true;

        while accept_loop {
            let semaphore = semaphore.clone();
            let connsig = connsig.clone();

            tokio::select! {
                (permit, event) = async {
                    let permit = semaphore.acquire_owned().await.unwrap();
                    (permit, tcp_listener.accept().await)
                } => {
                    match event {
                        Ok((stream, remote_addr)) => {
                            $handler(local_addr, remote_addr, stream, permit, connsig, "http");
                        },
                        Err(err) => {
                            log::info!("TCP handshake failed with error: {:?}", err);
                            drop(permit);
                        }
                    }
                },
                _ = $pysig.changed() => {
                    accept_loop = false;
                    connsig.notify_waiters();
                }
            }
        }
    }};

    (
        tls
        $listener:expr,
        $listener_cfg:expr,
        $pysig:expr,
        $backpressure:expr,
        $handler:expr
    ) => {{
        let (mut tls_listener, local_addr) = crate::tls::tls_listener($listener_cfg.into(), $listener).unwrap();
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new($backpressure));
        let connsig = std::sync::Arc::new(tokio::sync::Notify::new());
        let mut accept_loop = true;

        while accept_loop {
            let semaphore = semaphore.clone();
            let connsig = connsig.clone();

            tokio::select! {
                (permit, event) = async {
                    let permit = semaphore.acquire_owned().await.unwrap();
                    (permit, tls_listener.accept().await)
                } => {
                    match event {
                        Ok((stream, remote_addr)) => {
                            $handler(local_addr, remote_addr, stream, permit, connsig, "https")
                        },
                        Err(err) => {
                            log::info!("TLS handshake failed with {:?}", err);
                            drop(permit);
                        }
                    }
                },
                _ = $pysig.changed() => {
                    accept_loop = false;
                    connsig.notify_waiters();
                }
            }
        }
    }};
}

macro_rules! gen_accept {
    (
        plain
        auto
        $conn_method:ident,
        $ctx:expr,
        $target:expr,
        $svc:ident,
        $tcp_listener:expr,
        $pyrx:expr,
        $backpressure:expr,
        $rt:expr,
        $spawner:expr,
        $executor:expr,
        $http1_opts:expr,
        $http2_opts:expr,
        $http2_stream_wrapper:expr
    ) => {
        crate::workers::accept_loop!(
            plain
            $tcp_listener,
            (),
            $pyrx,
            $backpressure,
            crate::workers::connection_handler!(
                auto
                $ctx,
                $target,
                $svc,
                $rt,
                $spawner,
                $executor,
                $conn_method,
                $http1_opts,
                $http2_opts
            )
        )
    };

    (
        plain
        1
        $conn_method:ident,
        $ctx:expr,
        $target:expr,
        $svc:ident,
        $tcp_listener:expr,
        $pyrx:expr,
        $backpressure:expr,
        $rt:expr,
        $spawner:expr,
        $executor:expr,
        $http1_opts:expr,
        $http2_opts:expr,
        $http2_stream_wrapper:expr
    ) => {
        crate::workers::accept_loop!(
            plain
            $tcp_listener,
            (),
            $pyrx,
            $backpressure,
            crate::workers::connection_handler!(
                1
                $ctx,
                $target,
                $svc,
                $rt,
                $spawner,
                $conn_method,
                $http1_opts
            )
        )
    };

    (
        plain
        2
        $conn_method:ident,
        $ctx:expr,
        $target:expr,
        $svc:ident,
        $tcp_listener:expr,
        $pyrx:expr,
        $backpressure:expr,
        $rt:expr,
        $spawner:expr,
        $executor:expr,
        $http1_opts:expr,
        $http2_opts:expr,
        $http2_stream_wrapper:expr
    ) => {
        crate::workers::accept_loop!(
            plain
            $tcp_listener,
            (),
            $pyrx,
            $backpressure,
            crate::workers::connection_handler!(
                2
                $ctx,
                $target,
                $svc,
                $rt,
                $spawner,
                $executor,
                $http2_stream_wrapper,
                $http2_opts
            )
        )
    };

    (
        tls
        auto
        $conn_method:ident,
        $tls_config:expr,
        $ctx:expr,
        $target:expr,
        $svc:ident,
        $tcp_listener:expr,
        $pyrx:expr,
        $backpressure:expr,
        $rt:expr,
        $spawner:expr,
        $executor:expr,
        $http1_opts:expr,
        $http2_opts:expr,
        $http2_stream_wrapper:expr
    ) => {
        crate::workers::accept_loop!(
            tls
            $tcp_listener,
            $tls_config,
            $pyrx,
            $backpressure,
            crate::workers::connection_handler!(
                auto
                $ctx,
                $target,
                $svc,
                $rt,
                $spawner,
                $executor,
                $conn_method,
                $http1_opts,
                $http2_opts
            )
        )
    };

    (
        tls
        1
        $conn_method:ident,
        $tls_config:expr,
        $ctx:expr,
        $target:expr,
        $svc:ident,
        $tcp_listener:expr,
        $pyrx:expr,
        $backpressure:expr,
        $rt:expr,
        $spawner:expr,
        $executor:expr,
        $http1_opts:expr,
        $http2_opts:expr,
        $http2_stream_wrapper:expr
    ) => {
        crate::workers::accept_loop!(
            tls
            $tcp_listener,
            $tls_config,
            $pyrx,
            $backpressure,
            crate::workers::connection_handler!(
                1
                $ctx,
                $target,
                $svc,
                $rt,
                $spawner,
                $conn_method,
                $http1_opts
            )
        )
    };

    (
        tls
        2
        $conn_method:ident,
        $tls_config:expr,
        $ctx:expr,
        $target:expr,
        $svc:ident,
        $tcp_listener:expr,
        $pyrx:expr,
        $backpressure:expr,
        $rt:expr,
        $spawner:expr,
        $executor:expr,
        $http1_opts:expr,
        $http2_opts:expr,
        $http2_stream_wrapper:expr
    ) => {
        crate::workers::accept_loop!(
            tls
            $tcp_listener,
            $tls_config,
            $pyrx,
            $backpressure,
            crate::workers::connection_handler!(
                2
                $ctx,
                $target,
                $svc,
                $rt,
                $spawner,
                $executor,
                $http2_stream_wrapper,
                $http2_opts
            )
        )
    };
}

macro_rules! serve_mtr {
    ($func_name:ident, $http_mode:tt, $conn_method:ident, $target:expr, $ctx:ty, $svc:ident) => {
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
            #[allow(unused_variables)]
            let http1_opts = self.config.http1_opts.clone();
            #[allow(unused_variables)]
            let http2_opts = self.config.http2_opts.clone();
            let backpressure = self.config.backpressure.clone();

            let ctxw: Box<dyn crate::workers::WorkerCTX<CTX=$ctx>> = Box::new(crate::workers::Worker::new(<$ctx>::new(callback, self.config.static_files.clone())));
            let ctx = ctxw.get_ctx();

            let rtpyloop = std::sync::Arc::new(event_loop.clone().unbind());
            let rt = py.allow_threads(|| crate::runtime::init_runtime_mt(
                self.config.threads,
                self.config.blocking_threads,
                self.config.py_threads,
                self.config.py_threads_idle_timeout,
                rtpyloop,
            ));
            let rth = rt.handler();
            let tasks = tokio_util::task::TaskTracker::new();
            let mut srx = signal.get().rx.lock().unwrap().take().unwrap();

            let main_loop = crate::runtime::run_until_complete(rt, event_loop.clone(), async move {
                crate::workers::gen_accept!(
                    plain
                    $http_mode
                    $conn_method,
                    ctx,
                    $target,
                    $svc,
                    tcp_listener,
                    srx,
                    backpressure,
                    rth,
                    |task| tasks.spawn(task),
                    hyper_util::rt::TokioExecutor::new,
                    http1_opts,
                    http2_opts,
                    hyper_util::rt::TokioIo::new
                );

                log::info!("Stopping worker-{}", worker_id);

                tasks.close();
                tasks.wait().await;

                Python::with_gil(|_| drop(ctx));
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
    ($func_name:ident, $http_mode:tt, $conn_method:ident, $target:expr, $ctx:ty, $svc:ident) => {
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
            #[allow(unused_variables)]
            let http1_opts = self.config.http1_opts.clone();
            #[allow(unused_variables)]
            let http2_opts = self.config.http2_opts.clone();
            let backpressure = self.config.backpressure.clone();
            let tls_cfg = self.config.tls_cfg();

            let ctxw: Box<dyn crate::workers::WorkerCTX<CTX=$ctx>> = Box::new(crate::workers::Worker::new(<$ctx>::new(callback, self.config.static_files.clone())));
            let ctx = ctxw.get_ctx();

            let rtpyloop = std::sync::Arc::new(event_loop.clone().unbind());
            let rt = py.allow_threads(|| crate::runtime::init_runtime_mt(
                self.config.threads,
                self.config.blocking_threads,
                self.config.py_threads,
                self.config.py_threads_idle_timeout,
                rtpyloop,
            ));
            let rth = rt.handler();
            let tasks = tokio_util::task::TaskTracker::new();
            let mut srx = signal.get().rx.lock().unwrap().take().unwrap();

            let main_loop = crate::runtime::run_until_complete(rt, event_loop.clone(), async move {
                crate::workers::gen_accept!(
                    tls
                    $http_mode
                    $conn_method,
                    tls_cfg,
                    ctx,
                    $target,
                    $svc,
                    tcp_listener,
                    srx,
                    backpressure,
                    rth,
                    |task| tasks.spawn(task),
                    hyper_util::rt::TokioExecutor::new,
                    http1_opts,
                    http2_opts,
                    hyper_util::rt::TokioIo::new
                );

                log::info!("Stopping worker-{}", worker_id);

                tasks.close();
                tasks.wait().await;

                Python::with_gil(|_| drop(ctx));
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
    ($http_mode:tt, $conn_method:ident, $target:expr, $ctx:ty, $svc:ident, $self:expr, $callback:expr, $event_loop:expr, $wid:expr, $workers:expr, $srx:expr) => {
        let ctxw: Box<dyn crate::workers::WorkerCTX<CTX=$ctx>> = Box::new(crate::workers::Worker::new(<$ctx>::new($callback, $self.config.static_files.clone())));
        let ctx = ctxw.get_ctx();
        let py_loop = std::sync::Arc::new($event_loop.clone().unbind());

        for thread_id in 0..$self.config.threads {
            log::info!("Started worker-{} runtime-{}", $wid, thread_id + 1);

            let tcp_listener = $self.config.tcp_listener();
            #[allow(unused_variables)]
            let http1_opts = $self.config.http1_opts.clone();
            #[allow(unused_variables)]
            let http2_opts = $self.config.http2_opts.clone();
            let blocking_threads = $self.config.blocking_threads.clone();
            let py_threads = $self.config.py_threads.clone();
            let py_threads_idle_timeout = $self.config.py_threads_idle_timeout.clone();
            let backpressure = $self.config.backpressure.clone();
            let ctx = ctx.clone();
            let py_loop = py_loop.clone();
            let mut srx = $srx.clone();

            $workers.push(std::thread::spawn(move || {
                let rt =
                    crate::runtime::init_runtime_st(blocking_threads, py_threads, py_threads_idle_timeout, py_loop);
                let rth = rt.handler();
                let local = tokio::task::LocalSet::new();
                let tasks = tokio_util::task::TaskTracker::new();

                crate::runtime::block_on_local(&rt, local, async move {
                    crate::workers::gen_accept!(
                        plain
                        $http_mode
                        $conn_method,
                        ctx,
                        $target,
                        $svc,
                        tcp_listener,
                        srx,
                        backpressure,
                        rth,
                        |task| tasks.spawn_local(task),
                        crate::workers::WorkerExecutor::new,
                        http1_opts,
                        http2_opts,
                        |stream| { crate::io::IOTypeNotSend::new(hyper_util::rt::TokioIo::new(stream)) }
                    );

                    log::info!("Stopping worker-{} runtime-{}", $wid, thread_id + 1);

                    tasks.close();
                    tasks.wait().await;

                    Python::with_gil(|_| drop(ctx));
                });

                Python::with_gil(|_| drop(rt));
            }));
        }
    };
}

macro_rules! serve_str {
    ($func_name:ident, $http_mode:tt, $conn_method:ident, $target:expr, $ctx:ty, $svc:ident) => {
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
            crate::workers::serve_str_inner!(
                $http_mode,
                $conn_method,
                $target,
                $ctx,
                $svc,
                self,
                callback,
                event_loop,
                worker_id,
                workers,
                srx
            );

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
    ($http_mode:tt, $conn_method:ident, $target:expr, $ctx:ty, $svc:ident, $self:expr, $callback:expr, $event_loop:expr, $wid:expr, $workers:expr, $srx:expr) => {
        let ctxw: Box<dyn crate::workers::WorkerCTX<CTX=$ctx>> = Box::new(crate::workers::Worker::new(<$ctx>::new($callback, $self.config.static_files.clone())));
        let ctx = ctxw.get_ctx();
        let py_loop = std::sync::Arc::new($event_loop.clone().unbind());

        for thread_id in 0..$self.config.threads {
            log::info!("Started worker-{} runtime-{}", $wid, thread_id + 1);

            let tcp_listener = $self.config.tcp_listener();
            #[allow(unused_variables)]
            let http1_opts = $self.config.http1_opts.clone();
            #[allow(unused_variables)]
            let http2_opts = $self.config.http2_opts.clone();
            let tls_cfg = $self.config.tls_cfg();
            let blocking_threads = $self.config.blocking_threads.clone();
            let py_threads = $self.config.py_threads.clone();
            let py_threads_idle_timeout = $self.config.py_threads_idle_timeout.clone();
            let backpressure = $self.config.backpressure.clone();
            let ctx = ctx.clone();
            let py_loop = py_loop.clone();
            let mut srx = $srx.clone();

            $workers.push(std::thread::spawn(move || {
                let rt =
                    crate::runtime::init_runtime_st(blocking_threads, py_threads, py_threads_idle_timeout, py_loop);
                let rth = rt.handler();
                let local = tokio::task::LocalSet::new();
                let tasks = tokio_util::task::TaskTracker::new();

                crate::runtime::block_on_local(&rt, local, async move {
                    crate::workers::gen_accept!(
                        tls
                        $http_mode
                        $conn_method,
                        tls_cfg,
                        ctx,
                        $target,
                        $svc,
                        tcp_listener,
                        srx,
                        backpressure,
                        rth,
                        |task| tasks.spawn_local(task),
                        crate::workers::WorkerExecutor::new,
                        http1_opts,
                        http2_opts,
                        |stream| { crate::io::IOTypeNotSend::new(hyper_util::rt::TokioIo::new(stream)) }
                    );

                    log::info!("Stopping worker-{} runtime-{}", $wid, thread_id + 1);

                    tasks.close();
                    tasks.wait().await;

                    Python::with_gil(|_| drop(ctx));
                });

                Python::with_gil(|_| drop(rt));
            }));
        }
    };
}

macro_rules! serve_str_ssl {
    ($func_name:ident, $http_mode:tt, $conn_method:ident, $target:expr, $ctx:ty, $svc:ident) => {
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
            crate::workers::serve_str_ssl_inner!(
                $http_mode,
                $conn_method,
                $target,
                $ctx,
                $svc,
                self,
                callback,
                event_loop,
                worker_id,
                workers,
                srx
            );

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
    ($func_name:ident, $http_mode:tt, $conn_method:ident, $target:expr, $ctx:ty, $svc:ident) => {
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
            crate::workers::serve_str_inner!(
                $http_mode,
                $conn_method,
                $target,
                $ctx,
                $svc,
                self,
                callback,
                event_loop,
                worker_id,
                workers,
                srx
            );

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
    ($func_name:ident, $http_mode:tt, $conn_method:ident, $target:expr, $ctx:ty, $svc:ident) => {
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
            crate::workers::serve_str_ssl_inner!(
                $http_mode,
                $conn_method,
                $target,
                $ctx,
                $svc,
                self,
                callback,
                event_loop,
                worker_id,
                workers,
                srx
            );

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

macro_rules! gen_serve_methods {
    ($target:expr) => {
        crate::workers::serve_mtr!(
            _serve_mtr_http_plain_auto_base,
            auto,
            serve_connection,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_mtr!(
            _serve_mtr_http_plain_auto_file,
            auto,
            serve_connection,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_mtr!(
            _serve_mtr_http_plain_autou_base,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_mtr!(
            _serve_mtr_http_plain_autou_file,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_mtr!(
            _serve_mtr_http_plain_1_base,
            1,
            connection_builder_h1,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_mtr!(
            _serve_mtr_http_plain_1_file,
            1,
            connection_builder_h1,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_mtr!(
            _serve_mtr_http_plain_1u_base,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_mtr!(
            _serve_mtr_http_plain_1u_file,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_mtr!(
            _serve_mtr_http_plain_2_base,
            2,
            serve_connection,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_mtr!(
            _serve_mtr_http_plain_2_file,
            2,
            serve_connection,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_mtr_ssl!(
            _serve_mtr_http_tls_auto_base,
            auto,
            serve_connection,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_mtr_ssl!(
            _serve_mtr_http_tls_auto_file,
            auto,
            serve_connection,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_mtr_ssl!(
            _serve_mtr_http_tls_autou_base,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_mtr_ssl!(
            _serve_mtr_http_tls_autou_file,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_mtr_ssl!(
            _serve_mtr_http_tls_1_base,
            1,
            connection_builder_h1,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_mtr_ssl!(
            _serve_mtr_http_tls_1_file,
            1,
            connection_builder_h1,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_mtr_ssl!(
            _serve_mtr_http_tls_1u_base,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_mtr_ssl!(
            _serve_mtr_http_tls_1u_file,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_mtr_ssl!(
            _serve_mtr_http_tls_2_base,
            2,
            serve_connection,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_mtr_ssl!(
            _serve_mtr_http_tls_2_file,
            2,
            serve_connection,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_str!(
            _serve_str_http_plain_auto_base,
            auto,
            serve_connection,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_str!(
            _serve_str_http_plain_auto_file,
            auto,
            serve_connection,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_str!(
            _serve_str_http_plain_autou_base,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_str!(
            _serve_str_http_plain_autou_file,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_str!(
            _serve_str_http_plain_1_base,
            1,
            connection_builder_h1,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_str!(
            _serve_str_http_plain_1_file,
            1,
            connection_builder_h1,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_str!(
            _serve_str_http_plain_1u_base,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_str!(
            _serve_str_http_plain_1u_file,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_str!(
            _serve_str_http_plain_2_base,
            2,
            serve_connection,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_str!(
            _serve_str_http_plain_2_file,
            2,
            serve_connection,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_str_ssl!(
            _serve_str_http_tls_auto_base,
            auto,
            serve_connection,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_str_ssl!(
            _serve_str_http_tls_auto_file,
            auto,
            serve_connection,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_str_ssl!(
            _serve_str_http_tls_autou_base,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_str_ssl!(
            _serve_str_http_tls_autou_file,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_str_ssl!(
            _serve_str_http_tls_1_base,
            1,
            connection_builder_h1,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_str_ssl!(
            _serve_str_http_tls_1_file,
            1,
            connection_builder_h1,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_str_ssl!(
            _serve_str_http_tls_1u_base,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_str_ssl!(
            _serve_str_http_tls_1u_file,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_str_ssl!(
            _serve_str_http_tls_2_base,
            2,
            serve_connection,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_str_ssl!(
            _serve_str_http_tls_2_file,
            2,
            serve_connection,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_fut!(
            _serve_fut_http_plain_auto_base,
            auto,
            serve_connection,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_fut!(
            _serve_fut_http_plain_auto_file,
            auto,
            serve_connection,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_fut!(
            _serve_fut_http_plain_autou_base,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_fut!(
            _serve_fut_http_plain_autou_file,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_fut!(
            _serve_fut_http_plain_1_base,
            1,
            connection_builder_h1,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_fut!(
            _serve_fut_http_plain_1_file,
            1,
            connection_builder_h1,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_fut!(
            _serve_fut_http_plain_1u_base,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_fut!(
            _serve_fut_http_plain_1u_file,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_fut!(
            _serve_fut_http_plain_2_base,
            2,
            serve_connection,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_fut!(
            _serve_fut_http_plain_2_file,
            2,
            serve_connection,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_fut_ssl!(
            _serve_fut_http_tls_auto_base,
            auto,
            serve_connection,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_fut_ssl!(
            _serve_fut_http_tls_auto_file,
            auto,
            serve_connection,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_fut_ssl!(
            _serve_fut_http_tls_autou_base,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_fut_ssl!(
            _serve_fut_http_tls_autou_file,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_fut_ssl!(
            _serve_fut_http_tls_1_base,
            1,
            connection_builder_h1,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_fut_ssl!(
            _serve_fut_http_tls_1_file,
            1,
            connection_builder_h1,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_fut_ssl!(
            _serve_fut_http_tls_1u_base,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_fut_ssl!(
            _serve_fut_http_tls_1u_file,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_fut_ssl!(
            _serve_fut_http_tls_2_base,
            2,
            serve_connection,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_fut_ssl!(
            _serve_fut_http_tls_2_file,
            2,
            serve_connection,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
    };

    (ws $target:expr) => {
        crate::workers::serve_mtr!(
            _serve_mtr_ws_plain_autou_base,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_mtr!(
            _serve_mtr_ws_plain_autou_file,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_mtr!(
            _serve_mtr_ws_plain_1u_base,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_mtr!(
            _serve_mtr_ws_plain_1u_file,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_mtr_ssl!(
            _serve_mtr_ws_tls_autou_base,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_mtr_ssl!(
            _serve_mtr_ws_tls_autou_file,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_mtr_ssl!(
            _serve_mtr_ws_tls_1u_base,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_mtr_ssl!(
            _serve_mtr_ws_tls_1u_file,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_str!(
            _serve_str_ws_plain_autou_base,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_str!(
            _serve_str_ws_plain_autou_file,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_str!(
            _serve_str_ws_plain_1u_base,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_str!(
            _serve_str_ws_plain_1u_file,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_str_ssl!(
            _serve_str_ws_tls_autou_base,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_str_ssl!(
            _serve_str_ws_tls_autou_file,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_str_ssl!(
            _serve_str_ws_tls_1u_base,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_str_ssl!(
            _serve_str_ws_tls_1u_file,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_fut!(
            _serve_fut_ws_plain_autou_base,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_fut!(
            _serve_fut_ws_plain_autou_file,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_fut!(
            _serve_fut_ws_plain_1u_base,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_fut!(
            _serve_fut_ws_plain_1u_file,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_fut_ssl!(
            _serve_fut_ws_tls_autou_base,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_fut_ssl!(
            _serve_fut_ws_tls_autou_file,
            auto,
            serve_connection_with_upgrades,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
        crate::workers::serve_fut_ssl!(
            _serve_fut_ws_tls_1u_base,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXBase,
            service_app
        );
        crate::workers::serve_fut_ssl!(
            _serve_fut_ws_tls_1u_file,
            1,
            connection_builder_h1u,
            $target,
            crate::workers::WorkerCTXFiles,
            service_files
        );
    };
}

macro_rules! gen_serve_match {
    (mtr $self:expr, $py:expr, $callback:expr, $event_loop:expr, $signal:expr) => {
        match (
            &$self.config.http_mode[..],
            $self.config.tls_opts.is_some(),
            $self.config.websockets_enabled,
            $self.config.static_files.is_some(),
        ) {
            ("auto", false, false, false) => {
                $self._serve_mtr_http_plain_auto_base($py, $callback, $event_loop, $signal)
            }
            ("auto", false, false, true) => $self._serve_mtr_http_plain_auto_file($py, $callback, $event_loop, $signal),
            ("auto", false, true, false) => $self._serve_mtr_ws_plain_autou_base($py, $callback, $event_loop, $signal),
            ("auto", false, true, true) => $self._serve_mtr_ws_plain_autou_file($py, $callback, $event_loop, $signal),
            ("auto", true, false, false) => $self._serve_mtr_http_tls_auto_base($py, $callback, $event_loop, $signal),
            ("auto", true, false, true) => $self._serve_mtr_http_tls_auto_file($py, $callback, $event_loop, $signal),
            ("auto", true, true, false) => $self._serve_mtr_ws_tls_autou_base($py, $callback, $event_loop, $signal),
            ("auto", true, true, true) => $self._serve_mtr_ws_tls_autou_file($py, $callback, $event_loop, $signal),
            ("1", false, false, false) => $self._serve_mtr_http_plain_1_base($py, $callback, $event_loop, $signal),
            ("1", false, false, true) => $self._serve_mtr_http_plain_1_file($py, $callback, $event_loop, $signal),
            ("1", false, true, false) => $self._serve_mtr_ws_plain_1u_base($py, $callback, $event_loop, $signal),
            ("1", false, true, true) => $self._serve_mtr_ws_plain_1u_file($py, $callback, $event_loop, $signal),
            ("1", true, false, false) => $self._serve_mtr_http_tls_1_base($py, $callback, $event_loop, $signal),
            ("1", true, false, true) => $self._serve_mtr_http_tls_1_file($py, $callback, $event_loop, $signal),
            ("1", true, true, false) => $self._serve_mtr_ws_tls_1u_base($py, $callback, $event_loop, $signal),
            ("1", true, true, true) => $self._serve_mtr_ws_tls_1u_file($py, $callback, $event_loop, $signal),
            ("2", false, _, false) => $self._serve_mtr_http_plain_2_base($py, $callback, $event_loop, $signal),
            ("2", false, _, true) => $self._serve_mtr_http_plain_2_file($py, $callback, $event_loop, $signal),
            ("2", true, _, false) => $self._serve_mtr_http_tls_2_base($py, $callback, $event_loop, $signal),
            ("2", true, _, true) => $self._serve_mtr_http_tls_2_file($py, $callback, $event_loop, $signal),
            _ => unreachable!(),
        }
    };

    (str $self:expr, $callback:expr, $event_loop:expr, $signal:expr) => {
        match (
            &$self.config.http_mode[..],
            $self.config.tls_opts.is_some(),
            $self.config.websockets_enabled,
            $self.config.static_files.is_some(),
        ) {
            ("auto", false, false, false) => $self._serve_str_http_plain_auto_base($callback, $event_loop, $signal),
            ("auto", false, false, true) => $self._serve_str_http_plain_auto_file($callback, $event_loop, $signal),
            ("auto", false, true, false) => $self._serve_str_ws_plain_autou_base($callback, $event_loop, $signal),
            ("auto", false, true, true) => $self._serve_str_ws_plain_autou_file($callback, $event_loop, $signal),
            ("auto", true, false, false) => $self._serve_str_http_tls_auto_base($callback, $event_loop, $signal),
            ("auto", true, false, true) => $self._serve_str_http_tls_auto_file($callback, $event_loop, $signal),
            ("auto", true, true, false) => $self._serve_str_ws_tls_autou_base($callback, $event_loop, $signal),
            ("auto", true, true, true) => $self._serve_str_ws_tls_autou_file($callback, $event_loop, $signal),
            ("1", false, false, false) => $self._serve_str_http_plain_1_base($callback, $event_loop, $signal),
            ("1", false, false, true) => $self._serve_str_http_plain_1_file($callback, $event_loop, $signal),
            ("1", false, true, false) => $self._serve_str_ws_plain_1u_base($callback, $event_loop, $signal),
            ("1", false, true, true) => $self._serve_str_ws_plain_1u_file($callback, $event_loop, $signal),
            ("1", true, false, false) => $self._serve_str_http_tls_1_base($callback, $event_loop, $signal),
            ("1", true, false, true) => $self._serve_str_http_tls_1_file($callback, $event_loop, $signal),
            ("1", true, true, false) => $self._serve_str_ws_tls_1u_base($callback, $event_loop, $signal),
            ("1", true, true, true) => $self._serve_str_ws_tls_1u_file($callback, $event_loop, $signal),
            ("2", false, _, false) => $self._serve_str_http_plain_2_base($callback, $event_loop, $signal),
            ("2", false, _, true) => $self._serve_str_http_plain_2_file($callback, $event_loop, $signal),
            ("2", true, _, false) => $self._serve_str_http_tls_2_base($callback, $event_loop, $signal),
            ("2", true, _, true) => $self._serve_str_http_tls_2_file($callback, $event_loop, $signal),
            _ => unreachable!(),
        }
    };

    (fut $self:expr, $callback:expr, $event_loop:expr, $signal:expr) => {
        match (
            &$self.config.http_mode[..],
            $self.config.tls_opts.is_some(),
            $self.config.websockets_enabled,
            $self.config.static_files.is_some(),
        ) {
            ("auto", false, false, false) => $self._serve_fut_http_plain_auto_base($callback, $event_loop, $signal),
            ("auto", false, false, true) => $self._serve_fut_http_plain_auto_file($callback, $event_loop, $signal),
            ("auto", false, true, false) => $self._serve_fut_ws_plain_autou_base($callback, $event_loop, $signal),
            ("auto", false, true, true) => $self._serve_fut_ws_plain_autou_file($callback, $event_loop, $signal),
            ("auto", true, false, false) => $self._serve_fut_http_tls_auto_base($callback, $event_loop, $signal),
            ("auto", true, false, true) => $self._serve_fut_http_tls_auto_file($callback, $event_loop, $signal),
            ("auto", true, true, false) => $self._serve_fut_ws_tls_autou_base($callback, $event_loop, $signal),
            ("auto", true, true, true) => $self._serve_fut_ws_tls_autou_file($callback, $event_loop, $signal),
            ("1", false, false, false) => $self._serve_fut_http_plain_1_base($callback, $event_loop, $signal),
            ("1", false, false, true) => $self._serve_fut_http_plain_1_file($callback, $event_loop, $signal),
            ("1", false, true, false) => $self._serve_fut_ws_plain_1u_base($callback, $event_loop, $signal),
            ("1", false, true, true) => $self._serve_fut_ws_plain_1u_file($callback, $event_loop, $signal),
            ("1", true, false, false) => $self._serve_fut_http_tls_1_base($callback, $event_loop, $signal),
            ("1", true, false, true) => $self._serve_fut_http_tls_1_file($callback, $event_loop, $signal),
            ("1", true, true, false) => $self._serve_fut_ws_tls_1u_base($callback, $event_loop, $signal),
            ("1", true, true, true) => $self._serve_fut_ws_tls_1u_file($callback, $event_loop, $signal),
            ("2", false, _, false) => $self._serve_fut_http_plain_2_base($callback, $event_loop, $signal),
            ("2", false, _, true) => $self._serve_fut_http_plain_2_file($callback, $event_loop, $signal),
            ("2", true, _, false) => $self._serve_fut_http_tls_2_base($callback, $event_loop, $signal),
            ("2", true, _, true) => $self._serve_fut_http_tls_2_file($callback, $event_loop, $signal),
            _ => unreachable!(),
        }
    };
}

pub(crate) use accept_loop;
pub(crate) use build_service_fn;
pub(crate) use connection_builder_h1;
pub(crate) use connection_builder_h1u;
pub(crate) use connection_handler;
pub(crate) use gen_accept;
pub(crate) use gen_serve_match;
pub(crate) use gen_serve_methods;
pub(crate) use serve_fut;
pub(crate) use serve_fut_ssl;
pub(crate) use serve_mtr;
pub(crate) use serve_mtr_ssl;
pub(crate) use serve_str;
pub(crate) use serve_str_inner;
pub(crate) use serve_str_ssl;
pub(crate) use serve_str_ssl_inner;
pub(crate) use service_app;
pub(crate) use service_files;

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<WorkerSignal>()?;
    module.add_class::<WorkerSignalSync>()?;
    module.add_class::<ASGIWorker>()?;
    module.add_class::<RSGIWorker>()?;
    module.add_class::<WSGIWorker>()?;

    Ok(())
}
