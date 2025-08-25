use futures::FutureExt;
use pyo3::prelude::*;
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
};

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

struct WorkerSvcCtx {
    rt: crate::runtime::RuntimeRef,
    disconnect_guard: Arc<tokio::sync::Notify>,
    addr_local: crate::net::SockAddr,
    addr_remote: crate::net::SockAddr,
    proto: crate::http::HTTPProto,
}

trait SvcFnBuilder<F> {
    fn service(
        &self,
        ctx: WorkerSvcCtx,
    ) -> Box<
        dyn hyper::service::Service<
                crate::http::HTTPRequest,
                Response = crate::http::HTTPResponse,
                Error = std::convert::Infallible,
                Future = Pin<
                    Box<dyn Future<Output = Result<crate::http::HTTPResponse, std::convert::Infallible>> + Send>,
                >,
            > + Send
            + '_,
    >;
}

impl<A, H, F, Ret> SvcFnBuilder<F> for Worker<WorkerCTXBase, A, H, F>
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
        + 'static,
    Ret: Future<Output = crate::http::HTTPResponse> + Send + 'static,
    A: Send + Sync + 'static,
    H: Send + Sync + 'static,
{
    fn service(
        &self,
        ctx: WorkerSvcCtx,
    ) -> Box<
        dyn hyper::service::Service<
                crate::http::HTTPRequest,
                Response = crate::http::HTTPResponse,
                Error = std::convert::Infallible,
                Future = Pin<
                    Box<dyn Future<Output = Result<crate::http::HTTPResponse, std::convert::Infallible>> + Send>,
                >,
            > + Send,
    > {
        let f = self.target;
        let pycbs = self.ctx.callback.clone();
        Box::new(hyper::service::service_fn(move |req| {
            let fut = (f)(
                ctx.rt.clone(),
                ctx.disconnect_guard.clone(),
                pycbs.clone(),
                ctx.addr_local.clone(),
                ctx.addr_remote.clone(),
                req,
                ctx.proto.clone(),
            );

            async move { Ok::<_, std::convert::Infallible>(fut.await) }.boxed()
        }))
    }
}

impl<A, H, F, Ret> SvcFnBuilder<F> for Worker<WorkerCTXFiles, A, H, F>
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
        + Send
        + Sync
        + 'static,
    Ret: Future<Output = crate::http::HTTPResponse> + Send + 'static,
    A: Send + Sync + 'static,
    H: Send + Sync + 'static,
{
    fn service(
        &self,
        ctx: WorkerSvcCtx,
    ) -> Box<
        dyn hyper::service::Service<
                crate::http::HTTPRequest,
                Response = crate::http::HTTPResponse,
                Error = std::convert::Infallible,
                Future = Pin<
                    Box<dyn Future<Output = Result<crate::http::HTTPResponse, std::convert::Infallible>> + Send>,
                >,
            > + Send
            + '_,
    > {
        Box::new(hyper::service::service_fn(move |req| {
            if let Some(static_match) =
                crate::files::match_static_file(req.uri().path(), &self.ctx.static_prefix, &self.ctx.static_mount)
            {
                if static_match.is_err() {
                    return async move { Ok::<_, std::convert::Infallible>(crate::http::response_404()) }.boxed();
                }
                let expires = self.ctx.static_expires.clone();
                return async move {
                    Ok::<_, std::convert::Infallible>(
                        crate::files::serve_static_file(static_match.unwrap(), expires).await,
                    )
                }
                .boxed();
            }

            let pycbs = self.ctx.callback.clone();
            let fut = (self.target)(
                ctx.rt.clone(),
                ctx.disconnect_guard.clone(),
                pycbs,
                ctx.addr_local.clone(),
                ctx.addr_remote.clone(),
                req,
                ctx.proto.clone(),
            );

            async move { Ok::<_, std::convert::Infallible>(fut.await) }.boxed()
        }))
    }
}

#[derive(Clone)]
pub(crate) struct WorkerH1 {
    pub opts: HTTP1Config,
}

#[derive(Clone)]
pub(crate) struct WorkerH1U {
    pub opts: HTTP1Config,
}

#[derive(Clone)]
pub(crate) struct WorkerH2 {
    pub opts: HTTP2Config,
}

#[derive(Clone)]
pub(crate) struct WorkerHA {
    pub opts_h1: HTTP1Config,
    pub opts_h2: HTTP2Config,
}

#[derive(Clone)]
pub(crate) struct WorkerHAU {
    pub opts_h1: HTTP1Config,
    pub opts_h2: HTTP2Config,
}

trait WorkerConnectionHandler<S> {
    fn handle(
        self,
        addr_local: crate::net::SockAddr,
        addr_remote: crate::net::SockAddr,
        stream: S,
        permit: tokio::sync::OwnedSemaphorePermit,
        sig: Arc<tokio::sync::Notify>,
        proto: crate::http::HTTPProto,
    ) -> impl Future<Output = ()> + Send + 'static;
}

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

macro_rules! conn_handler_h1 {
    ($cb:tt) => {
        async fn handle(
            self,
            addr_local: crate::net::SockAddr,
            addr_remote: crate::net::SockAddr,
            stream: S,
            permit: tokio::sync::OwnedSemaphorePermit,
            sig: Arc<tokio::sync::Notify>,
            proto: crate::http::HTTPProto,
        ) {
            let disconnect_guard = Arc::new(tokio::sync::Notify::new());
            let svc_ctx = WorkerSvcCtx {
                rt: self.rt.clone(),
                disconnect_guard: disconnect_guard.clone(),
                addr_local,
                addr_remote,
                proto,
            };
            let mut done = false;
            let svc = self.service(svc_ctx);
            let conn = $cb!(self.handler.opts, hyper_util::rt::TokioIo::new(stream), svc);
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

            disconnect_guard.notify_one();
            drop(permit);
        }
    };
}

macro_rules! conn_handler_ha {
    ($conn_method:ident) => {
        async fn handle(
            self,
            addr_local: crate::net::SockAddr,
            addr_remote: crate::net::SockAddr,
            stream: S,
            permit: tokio::sync::OwnedSemaphorePermit,
            sig: Arc<tokio::sync::Notify>,
            proto: crate::http::HTTPProto,
        ) {
            let disconnect_guard = Arc::new(tokio::sync::Notify::new());
            let svc_ctx = WorkerSvcCtx {
                rt: self.rt.clone(),
                disconnect_guard: disconnect_guard.clone(),
                addr_local,
                addr_remote,
                proto,
            };
            let mut done = false;
            let svc = self.service(svc_ctx);
            let mut connb = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
            connb
                .http1()
                .timer(hyper_util::rt::tokio::TokioTimer::new())
                .header_read_timeout(self.handler.opts_h1.header_read_timeout)
                .keep_alive(self.handler.opts_h1.keep_alive)
                .max_buf_size(self.handler.opts_h1.max_buffer_size)
                .pipeline_flush(self.handler.opts_h1.pipeline_flush);
            connb
                .http2()
                .timer(hyper_util::rt::tokio::TokioTimer::new())
                .adaptive_window(self.handler.opts_h2.adaptive_window)
                .initial_connection_window_size(self.handler.opts_h2.initial_connection_window_size)
                .initial_stream_window_size(self.handler.opts_h2.initial_stream_window_size)
                .keep_alive_interval(self.handler.opts_h2.keep_alive_interval)
                .keep_alive_timeout(self.handler.opts_h2.keep_alive_timeout)
                .max_concurrent_streams(self.handler.opts_h2.max_concurrent_streams)
                .max_frame_size(self.handler.opts_h2.max_frame_size)
                .max_header_list_size(self.handler.opts_h2.max_headers_size)
                .max_send_buf_size(self.handler.opts_h2.max_send_buffer_size);
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

            disconnect_guard.notify_one();
            drop(permit);
        }
    };
}

macro_rules! conn_handler_impl {
    (h1 $handler:ty, $cb:tt) => {
        impl<C, A, F, Ret, S> WorkerConnectionHandler<S> for Worker<C, A, $handler, F>
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
                + Send
                + Sync
                + 'static,
            Ret: Future<Output = crate::http::HTTPResponse> + 'static,
            A: Send + Sync + 'static,
            C: Send + Sync + 'static,
            S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
            Worker<C, A, $handler, F>: SvcFnBuilder<F>,
        {
            conn_handler_h1!($cb);
        }
    };
    (ha $handler:ty, $conn_method:ident) => {
        impl<C, A, F, Ret, S> WorkerConnectionHandler<S> for Worker<C, A, $handler, F>
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
                + Send
                + Sync
                + 'static,
            Ret: Future<Output = crate::http::HTTPResponse> + 'static,
            A: Send + Sync + 'static,
            C: Send + Sync + 'static,
            S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
            Worker<C, A, $handler, F>: SvcFnBuilder<F>,
        {
            conn_handler_ha!($conn_method);
        }
    };
}

conn_handler_impl!(h1 WorkerH1, conn_builder_h1);
conn_handler_impl!(h1 WorkerH1U, conn_builder_h1u);
conn_handler_impl!(ha WorkerHA, serve_connection);
conn_handler_impl!(ha WorkerHAU, serve_connection_with_upgrades);

impl<C, A, F, Ret, S> WorkerConnectionHandler<S> for Worker<C, A, WorkerH2, F>
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
        + Send
        + Sync
        + 'static,
    Ret: Future<Output = crate::http::HTTPResponse> + 'static,
    A: Send + Sync + 'static,
    C: Send + Sync + 'static,
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    Worker<C, A, WorkerH2, F>: SvcFnBuilder<F>,
{
    async fn handle(
        self,
        addr_local: crate::net::SockAddr,
        addr_remote: crate::net::SockAddr,
        stream: S,
        permit: tokio::sync::OwnedSemaphorePermit,
        sig: Arc<tokio::sync::Notify>,
        proto: crate::http::HTTPProto,
    ) {
        let disconnect_guard = Arc::new(tokio::sync::Notify::new());
        let svc_ctx = WorkerSvcCtx {
            rt: self.rt.clone(),
            disconnect_guard: disconnect_guard.clone(),
            addr_local,
            addr_remote,
            proto,
        };
        let mut done = false;
        let svc = self.service(svc_ctx);
        let conn = hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
            .timer(hyper_util::rt::tokio::TokioTimer::new())
            .adaptive_window(self.handler.opts.adaptive_window)
            .initial_connection_window_size(self.handler.opts.initial_connection_window_size)
            .initial_stream_window_size(self.handler.opts.initial_stream_window_size)
            .keep_alive_interval(self.handler.opts.keep_alive_interval)
            .keep_alive_timeout(self.handler.opts.keep_alive_timeout)
            .max_concurrent_streams(self.handler.opts.max_concurrent_streams)
            .max_frame_size(self.handler.opts.max_frame_size)
            .max_header_list_size(self.handler.opts.max_headers_size)
            .max_send_buf_size(self.handler.opts.max_send_buffer_size)
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

        disconnect_guard.notify_one();
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
        self,
        sig: tokio::sync::watch::Receiver<bool>,
        listener: L,
        backpressure: usize,
    ) -> impl Future<Output = ()> + Send + 'static;
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
                + Send
                + Sync
                + 'static,
            Ret: Future<Output = crate::http::HTTPResponse> + 'static,
            C: Send + 'static,
            H: Send + 'static,
            Worker<C, $target_plain, H, F>: WorkerConnectionHandler<$stream> + Clone,
        {
            async fn listen(
                self,
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
                    let wrk = self.clone();
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
                                    let handler = wrk.clone().handle(
                                        addr_local.clone(),
                                        $sockwrap(addr_remote),
                                        stream,
                                        permit,
                                        connsig,
                                        crate::http::HTTPProto::Plain
                                    );
                                    wrk.tasks.spawn(handler);
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
                + Send
                + Sync
                + 'static,
            Ret: Future<Output = crate::http::HTTPResponse> + 'static,
            C: Send + 'static,
            H: Send + 'static,
            Worker<C, $target_tls, H, F>:
                WorkerConnectionHandler<tls_listener::rustls::server::TlsStream<$stream>> + Clone,
        {
            async fn listen(
                self,
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
                    let wrk = self.clone();
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
                                    let handler = wrk.clone().handle(
                                        addr_local.clone(),
                                        $sockwrap(addr_remote),
                                        stream,
                                        permit,
                                        connsig,
                                        crate::http::HTTPProto::Tls
                                    );
                                    wrk.tasks.spawn(handler);
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
            Worker<C, A, H, F>: WorkerAcceptor<$listener> + Clone + Send + 'static,
        {
            _ = pyo3_log::try_init();

            let worker_id = cfg.id;
            log::info!("Started worker-{worker_id}");

            let listener = cfg.$listener_gen();
            let backpressure = cfg.backpressure;

            let rtpyloop = Arc::new(event_loop.clone().unbind());
            let rt = py.allow_threads(|| {
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
            let srx = signal.get().rx.lock().unwrap().take().unwrap();

            let main_loop = crate::runtime::run_until_complete(rt, event_loop.clone(), async move {
                wrk.clone().listen(srx, listener, backpressure).await;

                log::info!("Stopping worker-{worker_id}");

                wrk.tasks.close();
                wrk.tasks.wait().await;

                Python::with_gil(|_| drop(wrk));
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
            Worker<C, A, H, F>: WorkerAcceptor<$listener> + Clone + Send + 'static,
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

                    crate::runtime::block_on_local(&rt, local, async move {
                        wrk.clone().listen(srx, listener, backpressure).await;

                        log::info!("Stopping worker-{} runtime-{}", worker_id, thread_id + 1);

                        wrk.tasks.close();
                        wrk.tasks.wait().await;

                        Python::with_gil(|_| drop(wrk));
                    });

                    Python::with_gil(|_| drop(rt));
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
            Worker<C, A, H, F>: WorkerAcceptor<$listener> + Clone + Send + 'static,
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

                rt.inner.block_on(async move {
                    wrk.clone().listen(srx, tcp_listener, backpressure).await;

                    log::info!("Stopping worker-{worker_id}");

                    wrk.tasks.close();
                    wrk.tasks.wait().await;

                    Python::with_gil(|_| drop(wrk));
                });

                Python::with_gil(|_| drop(rt));
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

                Python::with_gil(|py| {
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
                crate::workers::WorkerHA {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
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
                crate::workers::WorkerHA {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
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
                crate::workers::WorkerHAU {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
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
                crate::workers::WorkerHAU {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
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
                crate::workers::WorkerHA {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
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
                crate::workers::WorkerHA {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
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
                crate::workers::WorkerHAU {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
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
                crate::workers::WorkerHAU {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
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
                crate::workers::WorkerH1 {
                    opts: $self.config.http1_opts.clone(),
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
                crate::workers::WorkerH1 {
                    opts: $self.config.http1_opts.clone(),
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
                crate::workers::WorkerH1U {
                    opts: $self.config.http1_opts.clone(),
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
                crate::workers::WorkerH1U {
                    opts: $self.config.http1_opts.clone(),
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
                crate::workers::WorkerH1 {
                    opts: $self.config.http1_opts.clone(),
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
                crate::workers::WorkerH1 {
                    opts: $self.config.http1_opts.clone(),
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
                crate::workers::WorkerH1U {
                    opts: $self.config.http1_opts.clone(),
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
                crate::workers::WorkerH1U {
                    opts: $self.config.http1_opts.clone(),
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
                crate::workers::WorkerH2 {
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
                crate::workers::WorkerH2 {
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
                crate::workers::WorkerH2 {
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
                crate::workers::WorkerH2 {
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
