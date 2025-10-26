use pyo3::prelude::*;
use std::{
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex},
};

use super::asgi::serve::ASGIWorker;
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
    pub threads: usize,
    pub blocking_threads: usize,
    pub py_threads: usize,
    pub py_threads_idle_timeout: u64,
    pub backpressure: usize,
    pub http_mode: String,
    pub http1_opts: HTTP1Config,
    pub http2_opts: HTTP2Config,
    pub websockets_enabled: bool,
    pub static_files: Option<(String, String, Option<String>)>,
    pub tls_opts: Option<WorkerTlsConfig>,
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
        threads: usize,
        blocking_threads: usize,
        py_threads: usize,
        py_threads_idle_timeout: u64,
        backpressure: usize,
        http_mode: &str,
        http1_opts: HTTP1Config,
        http2_opts: HTTP2Config,
        websockets_enabled: bool,
        static_files: Option<(String, String, Option<String>)>,
        ssl_enabled: bool,
        ssl_cert: Option<String>,
        ssl_key: Option<String>,
        ssl_key_password: Option<String>,
        ssl_protocol_min: &str,
        ssl_ca: Option<String>,
        ssl_crl: Vec<String>,
        ssl_client_verify: bool,
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
pub(crate) struct WorkerCTXBase {
    pub callback: crate::callbacks::ArcCBScheduler,
}

impl WorkerCTXBase {
    pub fn new(callback: crate::callbacks::PyCBScheduler) -> Self {
        Self {
            callback: Arc::new(callback),
        }
    }
}

#[derive(Clone)]
pub(crate) struct WorkerCTXFiles {
    pub callback: crate::callbacks::ArcCBScheduler,
    pub static_prefix: String,
    pub static_mount: String,
    pub static_expires: Option<String>,
}

impl WorkerCTXFiles {
    pub fn new(callback: crate::callbacks::PyCBScheduler, files: Option<(String, String, Option<String>)>) -> Self {
        let (static_prefix, static_mount, static_expires) = files.unwrap();
        Self {
            callback: Arc::new(callback),
            static_prefix,
            static_mount,
            static_expires,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Worker<C, A, H, F> {
    ctx: C,
    acceptor: A,
    handler: H,
    rt: crate::runtime::RuntimeRef,
    pub tasks: tokio_util::task::TaskTracker,
    target: F,
}

impl<C, A, H, F, Ret> Worker<C, A, H, F>
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
    pub fn new(ctx: C, acceptor: A, handler: H, rt: crate::runtime::RuntimeRef, target: F) -> Self {
        Self {
            ctx,
            acceptor,
            handler,
            rt,
            tasks: tokio_util::task::TaskTracker::new(),
            target,
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

macro_rules! service_impl {
    ($marker:ty, $proto:expr) => {
        impl<F, Ret> hyper::service::Service<crate::http::HTTPRequest> for WorkerSvc<F, WorkerCTXBase, $marker>
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
                let fut = (self.f)(
                    self.rt.clone(),
                    self.disconnect_guard.clone(),
                    self.ctx.callback.clone(),
                    self.addr_local.clone(),
                    self.addr_remote.clone(),
                    req,
                    $proto,
                );
                Box::pin(async move { Ok::<_, hyper::Error>(fut.await) })
            }
        }

        impl<F, Ret> hyper::service::Service<crate::http::HTTPRequest> for WorkerSvc<F, WorkerCTXFiles, $marker>
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
                if let Some(static_match) =
                    crate::files::match_static_file(req.uri().path(), &self.ctx.static_prefix, &self.ctx.static_mount)
                {
                    if static_match.is_err() {
                        return Box::pin(async move { Ok::<_, hyper::Error>(crate::http::response_404()) });
                    }
                    let expires = self.ctx.static_expires.clone();
                    return Box::pin(async move {
                        Ok::<_, hyper::Error>(crate::files::serve_static_file(static_match.unwrap(), expires).await)
                    });
                }

                let fut = (self.f)(
                    self.rt.clone(),
                    self.disconnect_guard.clone(),
                    self.ctx.callback.clone(),
                    self.addr_local.clone(),
                    self.addr_remote.clone(),
                    req,
                    $proto,
                );
                Box::pin(async move { Ok::<_, hyper::Error>(fut.await) })
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
pub(crate) struct WorkerHandlerH1<U> {
    pub opts: HTTP1Config,
    pub _upgrades: PhantomData<U>,
}

#[derive(Clone)]
pub(crate) struct WorkerHandlerH2 {
    pub opts: HTTP2Config,
}

#[derive(Clone)]
pub(crate) struct WorkerHandlerHA<U> {
    pub opts_h1: HTTP1Config,
    pub opts_h2: HTTP2Config,
    pub _upgrades: PhantomData<U>,
}

struct WorkerHandleH1<U> {
    opts: HTTP1Config,
    guard: Arc<tokio::sync::Notify>,
    _upgrades: PhantomData<U>,
}

struct WorkerHandleH2 {
    opts: HTTP2Config,
    guard: Arc<tokio::sync::Notify>,
}

struct WorkerHandleHA<U> {
    opts_h1: HTTP1Config,
    opts_h2: HTTP2Config,
    guard: Arc<tokio::sync::Notify>,
    _upgrades: PhantomData<U>,
}

trait WorkerHandleBuilder<I, S> {
    fn handle(&self, guard: Arc<tokio::sync::Notify>) -> impl WorkerHandle<I, S>;
}

impl<C, A, F, I, S> WorkerHandleBuilder<I, S> for Worker<C, A, WorkerHandlerH1<WorkerMarkerConnNoUpgrades>, F>
where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    fn handle(&self, guard: Arc<tokio::sync::Notify>) -> impl WorkerHandle<I, S> {
        WorkerHandleH1 {
            opts: self.handler.opts.clone(),
            guard,
            _upgrades: PhantomData::<WorkerMarkerConnNoUpgrades>,
        }
    }
}

impl<C, A, F, I, S> WorkerHandleBuilder<I, S> for Worker<C, A, WorkerHandlerH1<WorkerMarkerConnUpgrades>, F>
where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    fn handle(&self, guard: Arc<tokio::sync::Notify>) -> impl WorkerHandle<I, S> {
        WorkerHandleH1 {
            opts: self.handler.opts.clone(),
            guard,
            _upgrades: PhantomData::<WorkerMarkerConnUpgrades>,
        }
    }
}

impl<C, A, F, I, S> WorkerHandleBuilder<I, S> for Worker<C, A, WorkerHandlerH2, F>
where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    fn handle(&self, guard: Arc<tokio::sync::Notify>) -> impl WorkerHandle<I, S> {
        WorkerHandleH2 {
            opts: self.handler.opts.clone(),
            guard,
        }
    }
}

impl<C, A, F, I, S> WorkerHandleBuilder<I, S> for Worker<C, A, WorkerHandlerHA<WorkerMarkerConnNoUpgrades>, F>
where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    fn handle(&self, guard: Arc<tokio::sync::Notify>) -> impl WorkerHandle<I, S> {
        WorkerHandleHA {
            opts_h1: self.handler.opts_h1.clone(),
            opts_h2: self.handler.opts_h2.clone(),
            guard,
            _upgrades: PhantomData::<WorkerMarkerConnNoUpgrades>,
        }
    }
}

impl<C, A, F, I, S> WorkerHandleBuilder<I, S> for Worker<C, A, WorkerHandlerHA<WorkerMarkerConnUpgrades>, F>
where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    fn handle(&self, guard: Arc<tokio::sync::Notify>) -> impl WorkerHandle<I, S> {
        WorkerHandleHA {
            opts_h1: self.handler.opts_h1.clone(),
            opts_h2: self.handler.opts_h2.clone(),
            guard,
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

macro_rules! conn_handle_h1 {
    ($cb:tt) => {
        async fn call(
            self,
            svc: S,
            stream: I,
            permit: tokio::sync::OwnedSemaphorePermit,
            sig: Arc<tokio::sync::Notify>,
        ) {
            let mut done = false;
            let conn = $cb!(self.opts, hyper_util::rt::TokioIo::new(stream), svc);
            tokio::pin!(conn);

            tokio::select! {
                biased;
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

            self.guard.notify_one();
            drop(permit);
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
            let mut done = false;
            let mut connb = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
            connb
                .http1()
                .timer(hyper_util::rt::tokio::TokioTimer::new())
                .header_read_timeout(self.opts_h1.header_read_timeout)
                .keep_alive(self.opts_h1.keep_alive)
                .max_buf_size(self.opts_h1.max_buffer_size)
                .pipeline_flush(self.opts_h1.pipeline_flush);
            connb
                .http2()
                .timer(hyper_util::rt::tokio::TokioTimer::new())
                .adaptive_window(self.opts_h2.adaptive_window)
                .initial_connection_window_size(self.opts_h2.initial_connection_window_size)
                .initial_stream_window_size(self.opts_h2.initial_stream_window_size)
                .keep_alive_interval(self.opts_h2.keep_alive_interval)
                .keep_alive_timeout(self.opts_h2.keep_alive_timeout)
                .max_concurrent_streams(self.opts_h2.max_concurrent_streams)
                .max_frame_size(self.opts_h2.max_frame_size)
                .max_header_list_size(self.opts_h2.max_headers_size)
                .max_send_buf_size(self.opts_h2.max_send_buffer_size);
            let conn = connb.$conn_method(hyper_util::rt::TokioIo::new(stream), svc);
            tokio::pin!(conn);

            tokio::select! {
                biased;
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

            self.guard.notify_one();
            drop(permit);
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

conn_handle_impl!(h1 WorkerHandleH1<WorkerMarkerConnNoUpgrades>, conn_builder_h1);
conn_handle_impl!(h1 WorkerHandleH1<WorkerMarkerConnUpgrades>, conn_builder_h1u);
conn_handle_impl!(ha WorkerHandleHA<WorkerMarkerConnNoUpgrades>, serve_connection);
conn_handle_impl!(ha WorkerHandleHA<WorkerMarkerConnUpgrades>, serve_connection_with_upgrades);

impl<I, S> WorkerHandle<I, S> for WorkerHandleH2
where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    S: hyper::service::Service<crate::http::HTTPRequest, Response = crate::http::HTTPResponse> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    async fn call(self, svc: S, stream: I, permit: tokio::sync::OwnedSemaphorePermit, sig: Arc<tokio::sync::Notify>) {
        let mut done = false;
        let conn = hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
            .timer(hyper_util::rt::tokio::TokioTimer::new())
            .adaptive_window(self.opts.adaptive_window)
            .initial_connection_window_size(self.opts.initial_connection_window_size)
            .initial_stream_window_size(self.opts.initial_stream_window_size)
            .keep_alive_interval(self.opts.keep_alive_interval)
            .keep_alive_timeout(self.opts.keep_alive_timeout)
            .max_concurrent_streams(self.opts.max_concurrent_streams)
            .max_frame_size(self.opts.max_frame_size)
            .max_header_list_size(self.opts.max_headers_size)
            .max_send_buf_size(self.opts.max_send_buffer_size)
            .serve_connection(hyper_util::rt::TokioIo::new(stream), svc);
        tokio::pin!(conn);

        tokio::select! {
            biased;
            _ = conn.as_mut() => {
                done = true;
            },
            () = sig.notified() => {
                conn.as_mut().graceful_shutdown();
            }
        }
        if !done {
            _ = conn.as_mut().await;
        }

        self.guard.notify_one();
        drop(permit);
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

macro_rules! acceptor_impl {
    ($target_plain:ty, $target_tls:ty, $listeneri:ty, $listenero:ty, $stream:ty, $tlswrap:expr, $sockwrap:expr) => {
        impl<C, H, F, Ret> WorkerAcceptor<$listeneri> for Worker<C, $target_plain, H, F>
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
            Worker<C, $target_plain, H, F>: WorkerHandleBuilder<$stream, WorkerSvc<F, C, WorkerMarkerPlain>> + Clone,
        {
            async fn listen(
                &self,
                mut sig: tokio::sync::watch::Receiver<bool>,
                listener: $listeneri,
                backpressure: usize,
            ) {
                let listener = <$listenero>::from_std(listener).unwrap();
                let addr_local = $sockwrap(listener.local_addr().unwrap());
                let semaphore = Arc::new(tokio::sync::Semaphore::new(backpressure));
                let connsig = Arc::new(tokio::sync::Notify::new());
                let mut accept_loop = true;

                while accept_loop {
                    let rt = self.rt.clone();
                    let tasks = self.tasks.clone();
                    let target = self.target;
                    let ctx = self.ctx.clone();
                    let semaphore = semaphore.clone();
                    let connsig = connsig.clone();

                    tokio::select! {
                        biased;
                        (permit, event) = async {
                            let permit = semaphore.acquire_owned().await.unwrap();
                            (permit, listener.accept().await)
                        } => {
                            match event {
                                Ok((stream, addr_remote)) => {
                                    let disconnect_guard = Arc::new(tokio::sync::Notify::new());
                                    let handle = self.handle(disconnect_guard.clone());
                                    let svc = WorkerSvc {
                                        f: target,
                                        ctx,
                                        rt,
                                        disconnect_guard,
                                        addr_local: addr_local.clone(),
                                        addr_remote: $sockwrap(addr_remote),
                                        _proto: PhantomData::<WorkerMarkerPlain>,
                                    };
                                    tasks.spawn(handle.call(svc, stream, permit, connsig));
                                },
                                Err(err) => {
                                    log::info!("TCP handshake failed with error: {err:?}");
                                    drop(permit);
                                }
                            }
                        },
                        _ = sig.changed() => {
                            accept_loop = false;
                            connsig.notify_waiters();
                        }
                    }
                }
            }
        }

        impl<C, H, F, Ret> WorkerAcceptor<$listeneri> for Worker<C, $target_tls, H, F>
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
            Worker<C, $target_tls, H, F>:
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
                let semaphore = Arc::new(tokio::sync::Semaphore::new(backpressure));
                let connsig = Arc::new(tokio::sync::Notify::new());
                let mut accept_loop = true;

                while accept_loop {
                    let rt = self.rt.clone();
                    let tasks = self.tasks.clone();
                    let target = self.target;
                    let ctx = self.ctx.clone();
                    let semaphore = semaphore.clone();
                    let connsig = connsig.clone();

                    tokio::select! {
                        biased;
                        (permit, event) = async {
                            let permit = semaphore.acquire_owned().await.unwrap();
                            (permit, tls_listener.accept().await)
                        } => {
                            match event {
                                Ok((stream, addr_remote)) => {
                                    let disconnect_guard = Arc::new(tokio::sync::Notify::new());
                                    let handle = self.handle(disconnect_guard.clone());
                                    let svc = WorkerSvc {
                                        f: target,
                                        ctx,
                                        rt,
                                        disconnect_guard,
                                        addr_local: addr_local.clone(),
                                        addr_remote: $sockwrap(addr_remote),
                                        _proto: PhantomData::<WorkerMarkerTls>,
                                    };
                                    tasks.spawn(handle.call(svc, stream, permit, connsig));
                                },
                                Err(err) => {
                                    log::info!("TCP handshake failed with error: {err:?}");
                                    drop(permit);
                                }
                            }
                        },
                        _ = sig.changed() => {
                            accept_loop = false;
                            connsig.notify_waiters();
                        }
                    }
                }
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

macro_rules! serve_fn {
    (mt $name:ident, $listener:ty, $listener_gen:ident) => {
        pub(crate) fn $name<C, A, H, F, Ret>(
            cfg: &WorkerConfig,
            py: Python,
            event_loop: &Bound<PyAny>,
            signal: Py<WorkerSignal>,
            ctx: C,
            acceptor: A,
            handler: H,
            target: F,
        ) where
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
            Worker<C, A, H, F>: WorkerAcceptor<$listener> + Send + 'static,
        {
            _ = pyo3_log::try_init();

            let worker_id = cfg.id;
            log::info!("Started worker-{worker_id}");

            let listener = cfg.$listener_gen();
            let backpressure = cfg.backpressure;

            let rtpyloop = Arc::new(event_loop.clone().unbind());
            let rt = py.detach(|| {
                crate::runtime::init_runtime_mt(
                    cfg.threads,
                    cfg.blocking_threads,
                    cfg.py_threads,
                    cfg.py_threads_idle_timeout,
                    rtpyloop,
                )
            });
            let rth = rt.handler();

            let wrk = crate::workers::Worker::new(ctx, acceptor, handler, rth, target);
            let tasks = wrk.tasks.clone();
            let srx = signal.get().rx.lock().unwrap().take().unwrap();

            let main_loop = crate::runtime::run_until_complete(rt, event_loop.clone(), async move {
                wrk.listen(srx, listener, backpressure).await;

                log::info!("Stopping worker-{worker_id}");

                tasks.close();
                tasks.wait().await;

                Python::attach(|_| drop(wrk));
                Ok(())
            });

            if let Err(err) = main_loop {
                log::error!("{err}");
                std::process::exit(1);
            }
        }
    };

    (st $name:ident, $listener:ty, $listener_gen:ident) => {
        pub(crate) fn $name<C, A, H, F, Ret>(
            cfg: &WorkerConfig,
            _py: (),
            event_loop: &Bound<PyAny>,
            signal: Py<WorkerSignal>,
            ctx: C,
            acceptor: A,
            handler: H,
            target: F,
        ) where
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
                + Send,
            Ret: Future<Output = crate::http::HTTPResponse>,
            C: Clone + Send + 'static,
            A: Clone + Send + 'static,
            H: Clone + Send + 'static,
            Worker<C, A, H, F>: WorkerAcceptor<$listener> + Send + 'static,
        {
            _ = pyo3_log::try_init();

            let worker_id = cfg.id;
            log::info!("Started worker-{worker_id}");

            let (stx, srx) = tokio::sync::watch::channel(false);
            let mut workers = vec![];

            let py_loop = Arc::new(event_loop.clone().unbind());

            for thread_id in 0..cfg.threads {
                log::info!("Started worker-{} runtime-{}", worker_id, thread_id + 1);

                let listener = cfg.$listener_gen();
                let blocking_threads = cfg.blocking_threads;
                let py_threads = cfg.py_threads;
                let py_threads_idle_timeout = cfg.py_threads_idle_timeout;
                let backpressure = cfg.backpressure;
                let ctx = ctx.clone();
                let acceptor = acceptor.clone();
                let handler = handler.clone();
                let target = target.clone();
                let py_loop = py_loop.clone();
                let srx = srx.clone();

                workers.push(std::thread::spawn(move || {
                    let rt =
                        crate::runtime::init_runtime_st(blocking_threads, py_threads, py_threads_idle_timeout, py_loop);
                    let rth = rt.handler();
                    let wrk = crate::workers::Worker::new(ctx, acceptor, handler, rth, target);
                    let local = tokio::task::LocalSet::new();
                    let tasks = wrk.tasks.clone();

                    crate::runtime::block_on_local(&rt, local, async move {
                        wrk.listen(srx, listener, backpressure).await;

                        log::info!("Stopping worker-{} runtime-{}", worker_id, thread_id + 1);

                        tasks.close();
                        tasks.wait().await;

                        Python::attach(|_| drop(wrk));
                    });

                    Python::attach(|_| drop(rt));
                }));
            }

            let rtm = crate::runtime::init_runtime_mt(1, 1, 0, 0, Arc::new(event_loop.clone().unbind()));
            let mut pyrx = signal.get().rx.lock().unwrap().take().unwrap();
            let main_loop = crate::runtime::run_until_complete(rtm, event_loop.clone(), async move {
                let _ = pyrx.changed().await;
                stx.send(true).unwrap();
                log::info!("Stopping worker-{worker_id}");
                while let Some(worker) = workers.pop() {
                    worker.join().unwrap();
                }
                Ok(())
            });

            if let Err(err) = main_loop {
                log::error!("{err}");
                std::process::exit(1);
            }
        }
    };

    (fut $name:ident, $listener:ty, $listener_gen:ident) => {
        pub(crate) fn $name<'p, C, A, H, F, Ret>(
            cfg: &WorkerConfig,
            _py: (),
            event_loop: &Bound<'p, PyAny>,
            signal: Py<WorkerSignal>,
            ctx: C,
            acceptor: A,
            handler: H,
            target: F,
        ) -> Bound<'p, PyAny>
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
                + Send,
            Ret: Future<Output = crate::http::HTTPResponse>,
            C: Clone + Send + 'static,
            A: Clone + Send + 'static,
            H: Clone + Send + 'static,
            Worker<C, A, H, F>: WorkerAcceptor<$listener> + Send + 'static,
        {
            _ = pyo3_log::try_init();

            let worker_id = cfg.id;
            log::info!("Started worker-{worker_id}");

            let tcp_listener = cfg.$listener_gen();
            let blocking_threads = cfg.blocking_threads;
            let py_threads = cfg.py_threads;
            let py_threads_idle_timeout = cfg.py_threads_idle_timeout;
            let backpressure = cfg.backpressure;

            let (stx, srx) = tokio::sync::watch::channel(false);
            let pyloop_r1 = Arc::new(event_loop.clone().unbind());
            let pyloop_r2 = pyloop_r1.clone();

            let worker = std::thread::spawn(move || {
                let rt =
                    crate::runtime::init_runtime_st(blocking_threads, py_threads, py_threads_idle_timeout, pyloop_r1);
                let rth = rt.handler();
                let wrk = crate::workers::Worker::new(ctx, acceptor, handler, rth, target);
                let tasks = wrk.tasks.clone();

                rt.inner.block_on(async move {
                    wrk.listen(srx, tcp_listener, backpressure).await;

                    log::info!("Stopping worker-{worker_id}");

                    tasks.close();
                    tasks.wait().await;

                    Python::attach(|_| drop(wrk));
                });

                Python::attach(|_| drop(rt));
            });

            let ret = event_loop.call_method0("create_future").unwrap();
            let pyfut = ret.clone().unbind();

            std::thread::spawn(move || {
                let rt = crate::runtime::init_runtime_st(1, 0, 0, pyloop_r2.clone());
                let local = tokio::task::LocalSet::new();

                let mut pyrx = signal.get().rx.lock().unwrap().take().unwrap();
                crate::runtime::block_on_local(&rt, local, async move {
                    let _ = pyrx.changed().await;
                    stx.send(true).unwrap();
                    log::info!("Stopping worker-{worker_id}");
                    worker.join().unwrap();
                });

                Python::attach(|py| {
                    let cb = pyfut.getattr(py, "set_result").unwrap();
                    _ = pyloop_r2.call_method1(
                        py,
                        "call_soon_threadsafe",
                        (crate::callbacks::PyFutureResultSetter, cb, py.None()),
                    );
                    drop(pyfut);
                    drop(pyloop_r2);
                    drop(signal);
                    drop(rt);
                });
            });

            ret
        }
    };
}

serve_fn!(mt serve_mt, std::net::TcpListener, tcp_listener);
serve_fn!(st serve_st, std::net::TcpListener, tcp_listener);
serve_fn!(fut serve_fut, std::net::TcpListener, tcp_listener);
#[cfg(unix)]
serve_fn!(mt serve_mt_uds, std::os::unix::net::UnixListener, uds_listener);
#[cfg(unix)]
serve_fn!(st serve_st_uds, std::os::unix::net::UnixListener, uds_listener);
#[cfg(unix)]
serve_fn!(fut serve_fut_uds, std::os::unix::net::UnixListener, uds_listener);

macro_rules! gen_serve_match {
    ($sm:expr, $acceptor_plain:ident, $acceptor_tls:ident, $self:expr, $py:expr, $callback:expr, $event_loop:expr, $signal:expr, $target:expr, $targetws:expr) => {
        match (
            &$self.config.http_mode[..],
            $self.config.tls_opts.is_some(),
            $self.config.websockets_enabled,
            $self.config.static_files.is_some(),
        ) {
            ("auto", false, false, false) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXBase::new($callback),
                crate::workers::$acceptor_plain {},
                crate::workers::WorkerHandlerHA {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnNoUpgrades>,
                },
                $target,
            ),
            ("auto", false, false, true) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXFiles::new($callback, $self.config.static_files.clone()),
                crate::workers::$acceptor_plain {},
                crate::workers::WorkerHandlerHA {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnNoUpgrades>,
                },
                $target,
            ),
            ("auto", false, true, false) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXBase::new($callback),
                crate::workers::$acceptor_plain {},
                crate::workers::WorkerHandlerHA {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnUpgrades>,
                },
                $targetws,
            ),
            ("auto", false, true, true) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXFiles::new($callback, $self.config.static_files.clone()),
                crate::workers::$acceptor_plain {},
                crate::workers::WorkerHandlerHA {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnUpgrades>,
                },
                $targetws,
            ),
            ("auto", true, false, false) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXBase::new($callback),
                crate::workers::$acceptor_tls {
                    opts: $self.config.tls_cfg().into(),
                },
                crate::workers::WorkerHandlerHA {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnNoUpgrades>,
                },
                $target,
            ),
            ("auto", true, false, true) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXFiles::new($callback, $self.config.static_files.clone()),
                crate::workers::$acceptor_tls {
                    opts: $self.config.tls_cfg().into(),
                },
                crate::workers::WorkerHandlerHA {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnNoUpgrades>,
                },
                $target,
            ),
            ("auto", true, true, false) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXBase::new($callback),
                crate::workers::$acceptor_tls {
                    opts: $self.config.tls_cfg().into(),
                },
                crate::workers::WorkerHandlerHA {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnUpgrades>,
                },
                $targetws,
            ),
            ("auto", true, true, true) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXFiles::new($callback, $self.config.static_files.clone()),
                crate::workers::$acceptor_tls {
                    opts: $self.config.tls_cfg().into(),
                },
                crate::workers::WorkerHandlerHA {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnUpgrades>,
                },
                $targetws,
            ),
            ("1", false, false, false) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXBase::new($callback),
                crate::workers::$acceptor_plain {},
                crate::workers::WorkerHandlerH1 {
                    opts: $self.config.http1_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnNoUpgrades>,
                },
                $target,
            ),
            ("1", false, false, true) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXFiles::new($callback, $self.config.static_files.clone()),
                crate::workers::$acceptor_plain {},
                crate::workers::WorkerHandlerH1 {
                    opts: $self.config.http1_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnNoUpgrades>,
                },
                $target,
            ),
            ("1", false, true, false) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXBase::new($callback),
                crate::workers::$acceptor_plain {},
                crate::workers::WorkerHandlerH1 {
                    opts: $self.config.http1_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnUpgrades>,
                },
                $targetws,
            ),
            ("1", false, true, true) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXFiles::new($callback, $self.config.static_files.clone()),
                crate::workers::$acceptor_plain {},
                crate::workers::WorkerHandlerH1 {
                    opts: $self.config.http1_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnUpgrades>,
                },
                $targetws,
            ),
            ("1", true, false, false) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXBase::new($callback),
                crate::workers::$acceptor_tls {
                    opts: $self.config.tls_cfg().into(),
                },
                crate::workers::WorkerHandlerH1 {
                    opts: $self.config.http1_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnNoUpgrades>,
                },
                $target,
            ),
            ("1", true, false, true) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXFiles::new($callback, $self.config.static_files.clone()),
                crate::workers::$acceptor_tls {
                    opts: $self.config.tls_cfg().into(),
                },
                crate::workers::WorkerHandlerH1 {
                    opts: $self.config.http1_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnNoUpgrades>,
                },
                $target,
            ),
            ("1", true, true, false) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXBase::new($callback),
                crate::workers::$acceptor_tls {
                    opts: $self.config.tls_cfg().into(),
                },
                crate::workers::WorkerHandlerH1 {
                    opts: $self.config.http1_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnUpgrades>,
                },
                $targetws,
            ),
            ("1", true, true, true) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXFiles::new($callback, $self.config.static_files.clone()),
                crate::workers::$acceptor_tls {
                    opts: $self.config.tls_cfg().into(),
                },
                crate::workers::WorkerHandlerH1 {
                    opts: $self.config.http1_opts.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnUpgrades>,
                },
                $targetws,
            ),
            ("2", false, _, false) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXBase::new($callback),
                crate::workers::$acceptor_plain {},
                crate::workers::WorkerHandlerH2 {
                    opts: $self.config.http2_opts.clone(),
                },
                $target,
            ),
            ("2", false, _, true) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXFiles::new($callback, $self.config.static_files.clone()),
                crate::workers::$acceptor_plain {},
                crate::workers::WorkerHandlerH2 {
                    opts: $self.config.http2_opts.clone(),
                },
                $target,
            ),
            ("2", true, _, false) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXBase::new($callback),
                crate::workers::$acceptor_tls {
                    opts: $self.config.tls_cfg().into(),
                },
                crate::workers::WorkerHandlerH2 {
                    opts: $self.config.http2_opts.clone(),
                },
                $target,
            ),
            ("2", true, _, true) => $sm(
                &$self.config,
                $py,
                $event_loop,
                $signal,
                crate::workers::WorkerCTXFiles::new($callback, $self.config.static_files.clone()),
                crate::workers::$acceptor_tls {
                    opts: $self.config.tls_cfg().into(),
                },
                crate::workers::WorkerHandlerH2 {
                    opts: $self.config.http2_opts.clone(),
                },
                $target,
            ),
            _ => unreachable!(),
        }
    };
}

pub(crate) use gen_serve_match;

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<WorkerSignal>()?;
    module.add_class::<WorkerSignalSync>()?;
    module.add_class::<ASGIWorker>()?;
    module.add_class::<RSGIWorker>()?;
    module.add_class::<WSGIWorker>()?;

    Ok(())
}
