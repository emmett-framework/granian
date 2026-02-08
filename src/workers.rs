use pyo3::prelude::*;
use std::{
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex},
};

use super::asgi::serve::ASGIWorker;
use super::metrics;
use super::rsgi::serve::RSGIWorker;
use super::tls::{
    load_certs as tls_load_certs, load_crls as tls_load_crls, load_private_key as tls_load_pkey,
    resolve_protocol_versions,
};
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
    pub qs: Py<PyAny>,
}

impl WorkerSignalSync {
    pub fn release(&self, py: Python) -> PyResult<Py<PyAny>> {
        self.qs.call_method0(py, "set")
    }
}

#[pymethods]
impl WorkerSignalSync {
    #[new]
    fn new(qs: Py<PyAny>) -> Self {
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
    sock: Py<crate::net::SocketHolder>,
    #[cfg(not(Py_GIL_DISABLED))]
    pub ipc: Option<Py<crate::ipc::IPCSenderHandle>>,
    pub threads: usize,
    pub blocking_threads: usize,
    pub py_threads: usize,
    pub py_threads_idle_timeout: u64,
    pub backpressure: usize,
    pub http_mode: String,
    pub http1_opts: HTTP1Config,
    pub http2_opts: HTTP2Config,
    pub websockets_enabled: bool,
    pub static_files: Option<(Vec<(String, String)>, Option<String>, Option<String>)>,
    pub tls_opts: Option<WorkerTlsConfig>,
    pub metrics: (
        Option<std::time::Duration>,
        Option<Py<crate::metrics::MetricsAggregator>>,
    ),
}

#[derive(Clone)]
pub(crate) struct WorkerTlsConfig {
    cert: String,
    key: (String, Option<String>),
    proto: String,
    ca: Option<String>,
    crl: Vec<String>,
    client_verify: bool,
}

impl WorkerConfig {
    pub fn new(
        id: i32,
        sock: Py<crate::net::SocketHolder>,
        #[allow(unused_variables)] ipc: Option<Py<crate::ipc::IPCSenderHandle>>,
        threads: usize,
        blocking_threads: usize,
        py_threads: usize,
        py_threads_idle_timeout: u64,
        backpressure: usize,
        http_mode: &str,
        http1_opts: HTTP1Config,
        http2_opts: HTTP2Config,
        websockets_enabled: bool,
        static_files: Option<(Vec<(String, String)>, Option<String>, Option<String>)>,
        ssl_enabled: bool,
        ssl_cert: Option<String>,
        ssl_key: Option<String>,
        ssl_key_password: Option<String>,
        ssl_protocol_min: &str,
        ssl_ca: Option<String>,
        ssl_crl: Vec<String>,
        ssl_client_verify: bool,
        metrics: (Option<u64>, Option<Py<crate::metrics::MetricsAggregator>>),
    ) -> Self {
        let tls_opts = match ssl_enabled {
            true => Some(WorkerTlsConfig {
                cert: ssl_cert.unwrap(),
                key: (ssl_key.unwrap(), ssl_key_password),
                proto: ssl_protocol_min.into(),
                ca: ssl_ca,
                crl: ssl_crl,
                client_verify: ssl_client_verify,
            }),
            false => None,
        };

        Self {
            id,
            sock,
            #[cfg(not(Py_GIL_DISABLED))]
            ipc,
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
            metrics: (metrics.0.map(std::time::Duration::from_secs), metrics.1),
        }
    }

    pub fn tcp_listener(&self) -> std::net::TcpListener {
        let listener = self.sock.get().as_tcp_listener().unwrap();
        _ = listener.set_nonblocking(true);
        listener
    }

    #[cfg(unix)]
    pub fn uds_listener(&self) -> std::os::unix::net::UnixListener {
        let listener = self.sock.get().as_unix_listener().unwrap();
        _ = listener.set_nonblocking(true);
        listener
    }

    pub fn tls_cfg(&self) -> tls_listener::rustls::rustls::ServerConfig {
        let opts = self.tls_opts.as_ref().unwrap();
        let tls_protos = resolve_protocol_versions(&opts.proto);

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
                tls_listener::rustls::rustls::ServerConfig::builder_with_protocol_versions(&tls_protos)
                    .with_client_cert_verifier(verifier)
            }
            None => tls_listener::rustls::rustls::ServerConfig::builder_with_protocol_versions(&tls_protos)
                .with_no_client_auth(),
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

struct WorkerMarkerPlain;
struct WorkerMarkerTls;

#[derive(Clone)]
pub(crate) struct WorkerMarkerConnNoUpgrades;

#[derive(Clone)]
pub(crate) struct WorkerMarkerConnUpgrades;

#[derive(Clone)]
pub(crate) struct WorkerCTXBase<M> {
    pub callback: crate::callbacks::ArcCBScheduler,
    pub metrics: M,
}

impl<M> WorkerCTXBase<M> {
    pub fn new(callback: crate::callbacks::PyCBScheduler, metrics: M) -> Self {
        Self {
            callback: Arc::new(callback),
            metrics,
        }
    }
}

#[derive(Clone)]
pub(crate) struct WorkerCTXFiles<M> {
    pub callback: crate::callbacks::ArcCBScheduler,
    pub metrics: M,
    pub static_mounts: Vec<(String, String)>,
    pub static_dir_to_file: Option<String>,
    pub static_expires: Option<String>,
}

impl<M> WorkerCTXFiles<M> {
    pub fn new(
        callback: crate::callbacks::PyCBScheduler,
        metrics: M,
        files: Option<(Vec<(String, String)>, Option<String>, Option<String>)>,
    ) -> Self {
        let (static_mounts, static_dir_to_file, static_expires) = files.unwrap();
        Self {
            callback: Arc::new(callback),
            metrics,
            static_mounts,
            static_dir_to_file,
            static_expires,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Worker<C, A, H, F, M> {
    ctx: C,
    acceptor: A,
    handler: H,
    pub rt: crate::runtime::RuntimeRef,
    pub tasks: tokio_util::task::TaskTracker,
    target: F,
    metrics: M,
}

impl<C, A, H, F, M, Ret> Worker<C, A, H, F, M>
where
    F: Fn(
            crate::runtime::RuntimeRef,
            Arc<tokio::sync::Notify>,
            crate::callbacks::ArcCBScheduler,
            crate::net::SockAddr,
            crate::net::SockAddr,
            crate::http::HTTPRequest,
            crate::http::HTTPProto,
        ) -> Ret
        + Copy,
    Ret: Future<Output = crate::http::HTTPResponse>,
{
    pub fn new(ctx: C, acceptor: A, handler: H, rt: crate::runtime::RuntimeRef, target: F, metrics: M) -> Self {
        Self {
            ctx,
            acceptor,
            handler,
            rt,
            tasks: tokio_util::task::TaskTracker::new(),
            target,
            metrics,
        }
    }
}

#[derive(Clone)]
struct WorkerSvc<F, C, P> {
    f: F,
    ctx: C,
    rt: crate::runtime::RuntimeRef,
    disconnect_guard: Arc<tokio::sync::Notify>,
    addr_local: crate::net::SockAddr,
    addr_remote: crate::net::SockAddr,
    _proto: PhantomData<P>,
}

macro_rules! service_proto_fut {
    ($proto:expr, $self:expr, $req:expr) => {{
        let fut = ($self.f)(
            $self.rt.clone(),
            $self.disconnect_guard.clone(),
            $self.ctx.callback.clone(),
            $self.addr_local.clone(),
            $self.addr_remote.clone(),
            $req,
            $proto,
        );
        Box::pin(async move { Ok::<_, hyper::Error>(fut.await) })
    }};
}

macro_rules! service_impl {
    ($proto_marker:ty, $proto:expr) => {
        impl<F, Ret> hyper::service::Service<crate::http::HTTPRequest>
            for WorkerSvc<F, WorkerCTXBase<()>, $proto_marker>
        where
            F: Fn(
                    crate::runtime::RuntimeRef,
                    Arc<tokio::sync::Notify>,
                    crate::callbacks::ArcCBScheduler,
                    crate::net::SockAddr,
                    crate::net::SockAddr,
                    crate::http::HTTPRequest,
                    crate::http::HTTPProto,
                ) -> Ret
                + Copy
                + Send
                + Sync
                + 'static,
            Ret: Future<Output = crate::http::HTTPResponse> + Send + 'static,
        {
            type Response = crate::http::HTTPResponse;
            type Error = hyper::Error;
            type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

            fn call(&self, req: crate::http::HTTPRequest) -> Self::Future {
                service_proto_fut!($proto, self, req)
            }
        }

        impl<F, Ret> hyper::service::Service<crate::http::HTTPRequest>
            for WorkerSvc<F, WorkerCTXFiles<()>, $proto_marker>
        where
            F: Fn(
                    crate::runtime::RuntimeRef,
                    Arc<tokio::sync::Notify>,
                    crate::callbacks::ArcCBScheduler,
                    crate::net::SockAddr,
                    crate::net::SockAddr,
                    crate::http::HTTPRequest,
                    crate::http::HTTPProto,
                ) -> Ret
                + Copy
                + Send
                + Sync
                + 'static,
            Ret: Future<Output = crate::http::HTTPResponse> + Send + 'static,
        {
            type Response = crate::http::HTTPResponse;
            type Error = hyper::Error;
            type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

            fn call(&self, req: crate::http::HTTPRequest) -> Self::Future {
                if let Some(static_match) = crate::files::match_static_file(
                    req.uri().path(),
                    &self.ctx.static_mounts,
                    self.ctx.static_dir_to_file.as_ref(),
                ) {
                    if static_match.is_err() {
                        return Box::pin(async move { Ok::<_, hyper::Error>(crate::http::response_404()) });
                    }
                    let expires = self.ctx.static_expires.clone();
                    return Box::pin(async move {
                        Ok::<_, hyper::Error>(crate::files::serve_static_file(static_match.unwrap(), expires).await)
                    });
                }

                service_proto_fut!($proto, self, req)
            }
        }

        impl<F, Ret> hyper::service::Service<crate::http::HTTPRequest>
            for WorkerSvc<F, WorkerCTXBase<crate::metrics::ArcWorkerMetrics>, $proto_marker>
        where
            F: Fn(
                    crate::runtime::RuntimeRef,
                    Arc<tokio::sync::Notify>,
                    crate::callbacks::ArcCBScheduler,
                    crate::net::SockAddr,
                    crate::net::SockAddr,
                    crate::http::HTTPRequest,
                    crate::http::HTTPProto,
                ) -> Ret
                + Copy
                + Send
                + Sync
                + 'static,
            Ret: Future<Output = crate::http::HTTPResponse> + Send + 'static,
        {
            type Response = crate::http::HTTPResponse;
            type Error = hyper::Error;
            type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

            fn call(&self, req: crate::http::HTTPRequest) -> Self::Future {
                self.ctx
                    .metrics
                    .req_handled
                    .fetch_add(1, std::sync::atomic::Ordering::Release);
                service_proto_fut!($proto, self, req)
            }
        }

        impl<F, Ret> hyper::service::Service<crate::http::HTTPRequest>
            for WorkerSvc<F, WorkerCTXFiles<crate::metrics::ArcWorkerMetrics>, $proto_marker>
        where
            F: Fn(
                    crate::runtime::RuntimeRef,
                    Arc<tokio::sync::Notify>,
                    crate::callbacks::ArcCBScheduler,
                    crate::net::SockAddr,
                    crate::net::SockAddr,
                    crate::http::HTTPRequest,
                    crate::http::HTTPProto,
                ) -> Ret
                + Copy
                + Send
                + Sync
                + 'static,
            Ret: Future<Output = crate::http::HTTPResponse> + Send + 'static,
        {
            type Response = crate::http::HTTPResponse;
            type Error = hyper::Error;
            type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

            fn call(&self, req: crate::http::HTTPRequest) -> Self::Future {
                self.ctx
                    .metrics
                    .req_handled
                    .fetch_add(1, std::sync::atomic::Ordering::Release);

                if let Some(static_match) = crate::files::match_static_file(
                    req.uri().path(),
                    &self.ctx.static_mounts,
                    self.ctx.static_dir_to_file.as_ref(),
                ) {
                    self.ctx
                        .metrics
                        .req_static_handled
                        .fetch_add(1, std::sync::atomic::Ordering::Release);
                    if static_match.is_err() {
                        self.ctx
                            .metrics
                            .req_static_err
                            .fetch_add(1, std::sync::atomic::Ordering::Release);
                        return Box::pin(async move { Ok::<_, hyper::Error>(crate::http::response_404()) });
                    }
                    let expires = self.ctx.static_expires.clone();
                    return Box::pin(async move {
                        Ok::<_, hyper::Error>(crate::files::serve_static_file(static_match.unwrap(), expires).await)
                    });
                }

                service_proto_fut!($proto, self, req)
            }
        }
    };
}

service_impl!(WorkerMarkerPlain, crate::http::HTTPProto::Plain);
service_impl!(WorkerMarkerTls, crate::http::HTTPProto::Tls);

macro_rules! conn_builder_h1 {
    ($opts:expr, $stream:expr, $svc:expr) => {
        hyper::server::conn::http1::Builder::new()
            .timer(hyper_util::rt::tokio::TokioTimer::new())
            .header_read_timeout($opts.header_read_timeout)
            .keep_alive($opts.keep_alive)
            .max_buf_size($opts.max_buffer_size)
            .pipeline_flush($opts.pipeline_flush)
            .serve_connection($stream, $svc)
    };
}

macro_rules! conn_builder_h1u {
    ($opts:expr, $stream:expr, $svc:expr) => {
        conn_builder_h1!($opts, $stream, $svc).with_upgrades()
    };
}

#[derive(Clone)]
pub(crate) struct WorkerHandlerH1<U, M> {
    pub opts: HTTP1Config,
    pub metrics: M,
    pub _upgrades: PhantomData<U>,
}

#[derive(Clone)]
pub(crate) struct WorkerHandlerH2<M> {
    pub opts: HTTP2Config,
    pub metrics: M,
}

#[derive(Clone)]
pub(crate) struct WorkerHandlerHA<U, M> {
    pub opts_h1: HTTP1Config,
    pub opts_h2: HTTP2Config,
    pub metrics: M,
    pub _upgrades: PhantomData<U>,
}

struct WorkerHandleH1<U, M> {
    opts: HTTP1Config,
    guard: Arc<tokio::sync::Notify>,
    metrics: M,
    _upgrades: PhantomData<U>,
}

struct WorkerHandleH2<M> {
    opts: HTTP2Config,
    guard: Arc<tokio::sync::Notify>,
    metrics: M,
}

struct WorkerHandleHA<U, M> {
    opts_h1: HTTP1Config,
    opts_h2: HTTP2Config,
    guard: Arc<tokio::sync::Notify>,
    metrics: M,
    _upgrades: PhantomData<U>,
}

trait WorkerHandleBuilder<I, S> {
    fn handle(&self, guard: Arc<tokio::sync::Notify>) -> impl WorkerHandle<I, S>;
}

impl<C, A, F, M, I, S> WorkerHandleBuilder<I, S> for Worker<C, A, WorkerHandlerH1<WorkerMarkerConnNoUpgrades, M>, F, M>
where
    M: Clone,
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    WorkerHandleH1<WorkerMarkerConnNoUpgrades, M>: WorkerHandle<I, S>,
{
    fn handle(&self, guard: Arc<tokio::sync::Notify>) -> impl WorkerHandle<I, S> {
        WorkerHandleH1 {
            opts: self.handler.opts.clone(),
            guard,
            metrics: self.handler.metrics.clone(),
            _upgrades: PhantomData::<WorkerMarkerConnNoUpgrades>,
        }
    }
}

impl<C, A, F, M, I, S> WorkerHandleBuilder<I, S> for Worker<C, A, WorkerHandlerH1<WorkerMarkerConnUpgrades, M>, F, M>
where
    M: Clone,
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    WorkerHandleH1<WorkerMarkerConnUpgrades, M>: WorkerHandle<I, S>,
{
    fn handle(&self, guard: Arc<tokio::sync::Notify>) -> impl WorkerHandle<I, S> {
        WorkerHandleH1 {
            opts: self.handler.opts.clone(),
            guard,
            metrics: self.handler.metrics.clone(),
            _upgrades: PhantomData::<WorkerMarkerConnUpgrades>,
        }
    }
}

impl<C, A, F, M, I, S> WorkerHandleBuilder<I, S> for Worker<C, A, WorkerHandlerH2<M>, F, M>
where
    M: Clone,
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    WorkerHandleH2<M>: WorkerHandle<I, S>,
{
    fn handle(&self, guard: Arc<tokio::sync::Notify>) -> impl WorkerHandle<I, S> {
        WorkerHandleH2 {
            opts: self.handler.opts.clone(),
            guard,
            metrics: self.handler.metrics.clone(),
        }
    }
}

impl<C, A, F, M, I, S> WorkerHandleBuilder<I, S> for Worker<C, A, WorkerHandlerHA<WorkerMarkerConnNoUpgrades, M>, F, M>
where
    M: Clone,
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    WorkerHandleHA<WorkerMarkerConnNoUpgrades, M>: WorkerHandle<I, S>,
{
    fn handle(&self, guard: Arc<tokio::sync::Notify>) -> impl WorkerHandle<I, S> {
        WorkerHandleHA {
            opts_h1: self.handler.opts_h1.clone(),
            opts_h2: self.handler.opts_h2.clone(),
            guard,
            metrics: self.handler.metrics.clone(),
            _upgrades: PhantomData::<WorkerMarkerConnNoUpgrades>,
        }
    }
}

impl<C, A, F, M, I, S> WorkerHandleBuilder<I, S> for Worker<C, A, WorkerHandlerHA<WorkerMarkerConnUpgrades, M>, F, M>
where
    M: Clone,
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    WorkerHandleHA<WorkerMarkerConnUpgrades, M>: WorkerHandle<I, S>,
{
    fn handle(&self, guard: Arc<tokio::sync::Notify>) -> impl WorkerHandle<I, S> {
        WorkerHandleHA {
            opts_h1: self.handler.opts_h1.clone(),
            opts_h2: self.handler.opts_h2.clone(),
            guard,
            metrics: self.handler.metrics.clone(),
            _upgrades: PhantomData::<WorkerMarkerConnUpgrades>,
        }
    }
}

trait WorkerHandle<I, S> {
    fn call(
        self,
        svc: S,
        stream: I,
        permit: tokio::sync::OwnedSemaphorePermit,
        sig: Arc<tokio::sync::Notify>,
    ) -> impl Future<Output = ()> + Send + 'static;
}

macro_rules! conn_handle_h1_impl {
    ($cb:tt, $self:expr, $svc:expr, $stream:expr, $permit:expr, $sig:expr) => {{
        let mut done = false;
        let conn = $cb!($self.opts, hyper_util::rt::TokioIo::new($stream), $svc);
        tokio::pin!(conn);

        tokio::select! {
            biased;
            _ = conn.as_mut() => {
                done = true;
            },
            _ = $sig.notified() => {
                conn.as_mut().graceful_shutdown();
            }
        }
        if !done {
            _ = conn.as_mut().await;
        }

        $self.guard.notify_one();
        drop($permit);
    }};
}

macro_rules! conn_handle_ha_impl {
    ($conn_method:ident, $self:expr, $svc:expr, $stream:expr, $permit:expr, $sig:expr) => {{
        let mut done = false;
        let mut connb = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
        connb
            .http1()
            .timer(hyper_util::rt::tokio::TokioTimer::new())
            .header_read_timeout($self.opts_h1.header_read_timeout)
            .keep_alive($self.opts_h1.keep_alive)
            .max_buf_size($self.opts_h1.max_buffer_size)
            .pipeline_flush($self.opts_h1.pipeline_flush);
        connb
            .http2()
            .timer(hyper_util::rt::tokio::TokioTimer::new())
            .adaptive_window($self.opts_h2.adaptive_window)
            .initial_connection_window_size($self.opts_h2.initial_connection_window_size)
            .initial_stream_window_size($self.opts_h2.initial_stream_window_size)
            .keep_alive_interval($self.opts_h2.keep_alive_interval)
            .keep_alive_timeout($self.opts_h2.keep_alive_timeout)
            .max_concurrent_streams($self.opts_h2.max_concurrent_streams)
            .max_frame_size($self.opts_h2.max_frame_size)
            .max_header_list_size($self.opts_h2.max_headers_size)
            .max_send_buf_size($self.opts_h2.max_send_buffer_size);
        let conn = connb.$conn_method(hyper_util::rt::TokioIo::new($stream), $svc);
        tokio::pin!(conn);

        tokio::select! {
            biased;
            _ = conn.as_mut() => {
                done = true;
            },
            _ = $sig.notified() => {
                conn.as_mut().graceful_shutdown();
            }
        }
        if !done {
            _ = conn.as_mut().await;
        }

        $self.guard.notify_one();
        drop($permit);
    }};
}

macro_rules! conn_handle_h2_impl {
    ($self:expr, $svc:expr, $stream:expr, $permit:expr, $sig:expr) => {{
        let mut done = false;
        let conn = hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
            .timer(hyper_util::rt::tokio::TokioTimer::new())
            .adaptive_window($self.opts.adaptive_window)
            .initial_connection_window_size($self.opts.initial_connection_window_size)
            .initial_stream_window_size($self.opts.initial_stream_window_size)
            .keep_alive_interval($self.opts.keep_alive_interval)
            .keep_alive_timeout($self.opts.keep_alive_timeout)
            .max_concurrent_streams($self.opts.max_concurrent_streams)
            .max_frame_size($self.opts.max_frame_size)
            .max_header_list_size($self.opts.max_headers_size)
            .max_send_buf_size($self.opts.max_send_buffer_size)
            .serve_connection(hyper_util::rt::TokioIo::new($stream), $svc);
        tokio::pin!(conn);

        tokio::select! {
            biased;
            _ = conn.as_mut() => {
                done = true;
            },
            () = $sig.notified() => {
                conn.as_mut().graceful_shutdown();
            }
        }
        if !done {
            _ = conn.as_mut().await;
        }

        $self.guard.notify_one();
        drop($permit);
    }};
}

macro_rules! conn_handle_h1 {
    ($cb:tt) => {
        async fn call(
            self,
            svc: S,
            stream: I,
            permit: tokio::sync::OwnedSemaphorePermit,
            sig: Arc<tokio::sync::Notify>,
        ) {
            conn_handle_h1_impl!($cb, self, svc, stream, permit, sig)
        }
    };
    (metrics $cb:tt) => {
        async fn call(
            self,
            svc: S,
            stream: I,
            permit: tokio::sync::OwnedSemaphorePermit,
            sig: Arc<tokio::sync::Notify>,
        ) {
            self.metrics
                .conn_active
                .fetch_add(1, std::sync::atomic::Ordering::Release);
            conn_handle_h1_impl!($cb, self, svc, stream, permit, sig);
            self.metrics
                .conn_active
                .fetch_sub(1, std::sync::atomic::Ordering::Release);
        }
    };
}

macro_rules! conn_handle_ha {
    ($conn_method:ident) => {
        async fn call(
            self,
            svc: S,
            stream: I,
            permit: tokio::sync::OwnedSemaphorePermit,
            sig: Arc<tokio::sync::Notify>,
        ) {
            conn_handle_ha_impl!($conn_method, self, svc, stream, permit, sig)
        }
    };
    (metrics $conn_method:ident) => {
        async fn call(
            self,
            svc: S,
            stream: I,
            permit: tokio::sync::OwnedSemaphorePermit,
            sig: Arc<tokio::sync::Notify>,
        ) {
            self.metrics
                .conn_active
                .fetch_add(1, std::sync::atomic::Ordering::Release);
            conn_handle_ha_impl!($conn_method, self, svc, stream, permit, sig);
            self.metrics
                .conn_active
                .fetch_sub(1, std::sync::atomic::Ordering::Release);
        }
    };
}

macro_rules! conn_handle_impl {
    (h1 $handle:ty, $cb:tt) => {
        impl<I, S> WorkerHandle<I, S> for $handle
        where
            I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
            S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
            S::Future: Send + 'static,
            S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        {
            conn_handle_h1!($cb);
        }
    };
    (ha $handle:ty, $conn_method:ident) => {
        impl<I, S> WorkerHandle<I, S> for $handle
        where
            I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
            S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
            S::Future: Send + 'static,
            S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        {
            conn_handle_ha!($conn_method);
        }
    };
}

macro_rules! conn_handle_metrics_impl {
    (h1 $handle:ty, $cb:tt) => {
        impl<I, S> WorkerHandle<I, S> for $handle
        where
            I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
            S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
            S::Future: Send + 'static,
            S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        {
            conn_handle_h1!(metrics $cb);
        }
    };
    (ha $handle:ty, $conn_method:ident) => {
        impl<I, S> WorkerHandle<I, S> for $handle
        where
            I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
            S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
            S::Future: Send + 'static,
            S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        {
            conn_handle_ha!(metrics $conn_method);
        }
    };
}

conn_handle_impl!(h1 WorkerHandleH1<WorkerMarkerConnNoUpgrades, ()>, conn_builder_h1);
conn_handle_impl!(h1 WorkerHandleH1<WorkerMarkerConnUpgrades, ()>, conn_builder_h1u);
conn_handle_impl!(ha WorkerHandleHA<WorkerMarkerConnNoUpgrades, ()>, serve_connection);
conn_handle_impl!(ha WorkerHandleHA<WorkerMarkerConnUpgrades, ()>, serve_connection_with_upgrades);
conn_handle_metrics_impl!(h1 WorkerHandleH1<WorkerMarkerConnNoUpgrades, metrics::ArcWorkerMetrics>, conn_builder_h1);
conn_handle_metrics_impl!(h1 WorkerHandleH1<WorkerMarkerConnUpgrades, metrics::ArcWorkerMetrics>, conn_builder_h1u);
conn_handle_metrics_impl!(ha WorkerHandleHA<WorkerMarkerConnNoUpgrades, metrics::ArcWorkerMetrics>, serve_connection);
conn_handle_metrics_impl!(ha WorkerHandleHA<WorkerMarkerConnUpgrades, metrics::ArcWorkerMetrics>, serve_connection_with_upgrades);

impl<I, S> WorkerHandle<I, S> for WorkerHandleH2<()>
where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    async fn call(self, svc: S, stream: I, permit: tokio::sync::OwnedSemaphorePermit, sig: Arc<tokio::sync::Notify>) {
        conn_handle_h2_impl!(self, svc, stream, permit, sig);
    }
}

impl<I, S> WorkerHandle<I, S> for WorkerHandleH2<metrics::ArcWorkerMetrics>
where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    async fn call(self, svc: S, stream: I, permit: tokio::sync::OwnedSemaphorePermit, sig: Arc<tokio::sync::Notify>) {
        self.metrics
            .conn_active
            .fetch_add(1, std::sync::atomic::Ordering::Release);
        conn_handle_h2_impl!(self, svc, stream, permit, sig);
        self.metrics
            .conn_active
            .fetch_sub(1, std::sync::atomic::Ordering::Release);
    }
}

#[derive(Clone)]
pub(crate) struct WorkerAcceptorTcpPlain {}

#[derive(Clone)]
pub(crate) struct WorkerAcceptorTcpTls {
    pub opts: Arc<tls_listener::rustls::rustls::ServerConfig>,
}

#[cfg(unix)]
#[derive(Clone)]
pub(crate) struct WorkerAcceptorUdsPlain {}

#[cfg(unix)]
#[derive(Clone)]
pub(crate) struct WorkerAcceptorUdsTls {
    pub opts: Arc<tls_listener::rustls::rustls::ServerConfig>,
}

pub(crate) trait WorkerAcceptor<L> {
    fn listen(
        &self,
        sig: tokio::sync::watch::Receiver<bool>,
        listener: L,
        backpressure: usize,
    ) -> impl Future<Output = ()> + Send;
}

macro_rules! acceptor_impl_stream {
    ($proto_marker:ty, $sockwrap:expr, $stream:expr, $addr_remote:expr, $self:expr, $addr_local:expr, $rt:expr, $tasks:expr, $permit:expr, $connsig:expr, $target:expr, $ctx:expr) => {{
        let disconnect_guard = Arc::new(tokio::sync::Notify::new());
        let handle = $self.handle(disconnect_guard.clone());
        let svc = WorkerSvc {
            f: $target,
            ctx: $ctx,
            rt: $rt,
            disconnect_guard,
            addr_local: $addr_local.clone(),
            addr_remote: $sockwrap($addr_remote),
            _proto: PhantomData::<$proto_marker>,
        };
        $tasks.spawn(handle.call(svc, $stream, $permit, $connsig));
    }};
}

macro_rules! acceptor_impl_err {
    ($err:expr, $permit:expr) => {{
        log::info!("TCP handshake failed with error: {:?}", $err);
        drop($permit);
    }};
}

macro_rules! acceptor_impl_match {
    ($proto_marker:ty, $sockwrap:expr, $event:expr, $self:expr, $addr_local:expr, $rt:expr, $tasks:expr, $permit:expr, $connsig:expr, $target:expr, $ctx:expr) => {{
        match $event {
            Ok((stream, addr_remote)) => acceptor_impl_stream!(
                $proto_marker,
                $sockwrap,
                stream,
                addr_remote,
                $self,
                $addr_local,
                $rt,
                $tasks,
                $permit,
                $connsig,
                $target,
                $ctx
            ),
            Err(err) => acceptor_impl_err!(err, $permit),
        }
    }};
}

macro_rules! acceptor_impl_match_metrics {
    ($proto_marker:ty, $sockwrap:expr, $event:expr, $self:expr, $addr_local:expr, $rt:expr, $tasks:expr, $permit:expr, $connsig:expr, $target:expr, $ctx:expr) => {{
        match $event {
            Ok((stream, addr_remote)) => {
                $self
                    .metrics
                    .conn_handled
                    .fetch_add(1, std::sync::atomic::Ordering::Release);
                acceptor_impl_stream!(
                    $proto_marker,
                    $sockwrap,
                    stream,
                    addr_remote,
                    $self,
                    $addr_local,
                    $rt,
                    $tasks,
                    $permit,
                    $connsig,
                    $target,
                    $ctx
                )
            }
            Err(err) => {
                $self
                    .metrics
                    .conn_err
                    .fetch_add(1, std::sync::atomic::Ordering::Release);
                acceptor_impl_err!(err, $permit)
            }
        }
    }};
}

macro_rules! acceptor_impl_loop {
    ($proto_marker:ty, $sockwrap:expr, $matchi:ident, $self:expr, $sig:expr, $backpressure:expr, $listener:expr, $addr_local:expr) => {{
        let semaphore = Arc::new(tokio::sync::Semaphore::new($backpressure));
        let connsig = Arc::new(tokio::sync::Notify::new());
        let mut accept_loop = true;

        while accept_loop {
            let rt = $self.rt.clone();
            let tasks = $self.tasks.clone();
            let target = $self.target;
            let ctx = $self.ctx.clone();
            let semaphore = semaphore.clone();
            let connsig = connsig.clone();

            tokio::select! {
                biased;
                (permit, event) = async {
                    let permit = semaphore.acquire_owned().await.unwrap();
                    (permit, $listener.accept().await)
                } => $matchi!(
                    $proto_marker,
                    $sockwrap,
                    event,
                    $self,
                    $addr_local,
                    rt,
                    tasks,
                    permit,
                    connsig,
                    target,
                    ctx
                ),
                _ = $sig.changed() => {
                    accept_loop = false;
                    connsig.notify_waiters();
                }
            }
        }
    }};
}

macro_rules! acceptor_impl {
    ($target_plain:ty, $target_tls:ty, $listeneri:ty, $listenero:ty, $stream:ty, $tlswrap:expr, $sockwrap:expr) => {
        impl<C, H, F, Ret> WorkerAcceptor<$listeneri> for Worker<C, $target_plain, H, F, ()>
        where
            F: Fn(
                    crate::runtime::RuntimeRef,
                    Arc<tokio::sync::Notify>,
                    crate::callbacks::ArcCBScheduler,
                    crate::net::SockAddr,
                    crate::net::SockAddr,
                    crate::http::HTTPRequest,
                    crate::http::HTTPProto,
                ) -> Ret
                + Copy
                + Send
                + Sync
                + 'static,
            Ret: Future<Output = crate::http::HTTPResponse> + 'static,
            C: Clone + Send + Sync + 'static,
            H: Send + Sync + 'static,
            Worker<C, $target_plain, H, F, ()>: WorkerHandleBuilder<$stream, WorkerSvc<F, C, WorkerMarkerPlain>> + Clone,
        {
            async fn listen(
                &self,
                mut sig: tokio::sync::watch::Receiver<bool>,
                listener: $listeneri,
                backpressure: usize,
            ) {
                let listener = <$listenero>::from_std(listener).unwrap();
                let addr_local = $sockwrap(listener.local_addr().unwrap());

                acceptor_impl_loop!(WorkerMarkerPlain, $sockwrap, acceptor_impl_match, self, sig, backpressure, listener, addr_local)
            }
        }

        impl<C, H, F, Ret> WorkerAcceptor<$listeneri> for Worker<C, $target_tls, H, F, ()>
        where
            F: Fn(
                    crate::runtime::RuntimeRef,
                    Arc<tokio::sync::Notify>,
                    crate::callbacks::ArcCBScheduler,
                    crate::net::SockAddr,
                    crate::net::SockAddr,
                    crate::http::HTTPRequest,
                    crate::http::HTTPProto,
                ) -> Ret
                + Copy
                + Send
                + Sync
                + 'static,
            Ret: Future<Output = crate::http::HTTPResponse> + 'static,
            C: Clone + Send + Sync + 'static,
            H: Send + Sync + 'static,
            Worker<C, $target_tls, H, F, ()>:
                WorkerHandleBuilder<tls_listener::rustls::server::TlsStream<$stream>, WorkerSvc<F, C, WorkerMarkerTls>> + Clone,
        {
            async fn listen(
                &self,
                mut sig: tokio::sync::watch::Receiver<bool>,
                listener: $listeneri,
                backpressure: usize,
            ) {
                let tls_cfg = self.acceptor.opts.clone();
                let (mut tls_listener, addr_local) = $tlswrap(tls_cfg, listener).unwrap();

                acceptor_impl_loop!(WorkerMarkerTls, $sockwrap, acceptor_impl_match, self, sig, backpressure, tls_listener, addr_local)
            }
        }

        impl<C, H, F, Ret> WorkerAcceptor<$listeneri> for Worker<C, $target_plain, H, F, crate::metrics::ArcWorkerMetrics>
        where
            F: Fn(
                    crate::runtime::RuntimeRef,
                    Arc<tokio::sync::Notify>,
                    crate::callbacks::ArcCBScheduler,
                    crate::net::SockAddr,
                    crate::net::SockAddr,
                    crate::http::HTTPRequest,
                    crate::http::HTTPProto,
                ) -> Ret
                + Copy
                + Send
                + Sync
                + 'static,
            Ret: Future<Output = crate::http::HTTPResponse> + 'static,
            C: Clone + Send + Sync + 'static,
            H: Send + Sync + 'static,
            Worker<C, $target_plain, H, F, crate::metrics::ArcWorkerMetrics>: WorkerHandleBuilder<$stream, WorkerSvc<F, C, WorkerMarkerPlain>> + Clone,
        {
            async fn listen(
                &self,
                mut sig: tokio::sync::watch::Receiver<bool>,
                listener: $listeneri,
                backpressure: usize,
            ) {
                let listener = <$listenero>::from_std(listener).unwrap();
                let addr_local = $sockwrap(listener.local_addr().unwrap());

                acceptor_impl_loop!(WorkerMarkerPlain, $sockwrap, acceptor_impl_match_metrics, self, sig, backpressure, listener, addr_local)
            }
        }

        impl<C, H, F, Ret> WorkerAcceptor<$listeneri> for Worker<C, $target_tls, H, F, crate::metrics::ArcWorkerMetrics>
        where
            F: Fn(
                    crate::runtime::RuntimeRef,
                    Arc<tokio::sync::Notify>,
                    crate::callbacks::ArcCBScheduler,
                    crate::net::SockAddr,
                    crate::net::SockAddr,
                    crate::http::HTTPRequest,
                    crate::http::HTTPProto,
                ) -> Ret
                + Copy
                + Send
                + Sync
                + 'static,
            Ret: Future<Output = crate::http::HTTPResponse> + 'static,
            C: Clone + Send + Sync + 'static,
            H: Send + Sync + 'static,
            Worker<C, $target_tls, H, F, crate::metrics::ArcWorkerMetrics>:
                WorkerHandleBuilder<tls_listener::rustls::server::TlsStream<$stream>, WorkerSvc<F, C, WorkerMarkerTls>> + Clone,
        {
            async fn listen(
                &self,
                mut sig: tokio::sync::watch::Receiver<bool>,
                listener: $listeneri,
                backpressure: usize,
            ) {
                let tls_cfg = self.acceptor.opts.clone();
                let (mut tls_listener, addr_local) = $tlswrap(tls_cfg, listener).unwrap();

                acceptor_impl_loop!(WorkerMarkerTls, $sockwrap, acceptor_impl_match_metrics, self, sig, backpressure, tls_listener, addr_local)
            }
        }
    };
}

acceptor_impl!(
    WorkerAcceptorTcpPlain,
    WorkerAcceptorTcpTls,
    std::net::TcpListener,
    tokio::net::TcpListener,
    tokio::net::TcpStream,
    crate::tls::tls_tcp_listener,
    crate::net::SockAddr::TCP
);
#[cfg(unix)]
acceptor_impl!(
    WorkerAcceptorUdsPlain,
    WorkerAcceptorUdsTls,
    std::os::unix::net::UnixListener,
    tokio::net::UnixListener,
    tokio::net::UnixStream,
    crate::tls::tls_uds_listener,
    crate::net::SockAddr::UDS
);

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<WorkerSignal>()?;
    module.add_class::<WorkerSignalSync>()?;
    module.add_class::<ASGIWorker>()?;
    module.add_class::<RSGIWorker>()?;
    module.add_class::<WSGIWorker>()?;

    Ok(())
}
