use pyo3::prelude::*;
use tokio::task::JoinHandle;

use super::http::handle;

use crate::callbacks::CallbackScheduler;
use crate::conversion::{worker_http1_config_from_py, worker_http2_config_from_py};
use crate::tcp::SocketHolder;
use crate::workers::{
    Worker, WorkerAcceptor, WorkerConfig, WorkerExcLocal, WorkerExcSend, WorkerSignalSync, gen_serve_match,
};

#[pyclass(frozen, module = "granian._granian")]
pub struct WSGIWorker {
    config: WorkerConfig,
}

#[pymethods]
impl WSGIWorker {
    #[new]
    #[pyo3(
        signature = (
            worker_id,
            sock,
            threads=1,
            blocking_threads=512,
            py_threads=1,
            py_threads_idle_timeout=30,
            backpressure=128,
            http_mode="1",
            http1_opts=None,
            http2_opts=None,
            static_files=None,
            ssl_enabled=false,
            ssl_cert=None,
            ssl_key=None,
            ssl_key_password=None,
            ssl_ca=None,
            ssl_crl=vec![],
            ssl_client_verify=false,
        )
    )]
    fn new(
        py: Python,
        worker_id: i32,
        sock: Py<SocketHolder>,
        threads: usize,
        blocking_threads: usize,
        py_threads: usize,
        py_threads_idle_timeout: u64,
        backpressure: usize,
        http_mode: &str,
        http1_opts: Option<PyObject>,
        http2_opts: Option<PyObject>,
        static_files: Option<(String, String, String)>,
        ssl_enabled: bool,
        ssl_cert: Option<String>,
        ssl_key: Option<String>,
        ssl_key_password: Option<String>,
        ssl_ca: Option<String>,
        ssl_crl: Vec<String>,
        ssl_client_verify: bool,
    ) -> PyResult<Self> {
        Ok(Self {
            config: WorkerConfig::new(
                worker_id,
                sock,
                threads,
                blocking_threads,
                py_threads,
                py_threads_idle_timeout,
                backpressure,
                http_mode,
                worker_http1_config_from_py(py, http1_opts)?,
                worker_http2_config_from_py(py, http2_opts)?,
                false,
                static_files,
                ssl_enabled,
                ssl_cert,
                ssl_key,
                ssl_key_password,
                ssl_ca,
                ssl_crl,
                ssl_client_verify,
            ),
        })
    }

    fn serve_mtr(
        &self,
        py: Python,
        callback: Py<CallbackScheduler>,
        event_loop: &Bound<PyAny>,
        signal: Py<WorkerSignalSync>,
    ) {
        gen_serve_match!(mt serve_mt, self, py, callback, event_loop, signal, handle, handle);
    }

    fn serve_str(
        &self,
        py: Python,
        callback: Py<CallbackScheduler>,
        event_loop: &Bound<PyAny>,
        signal: Py<WorkerSignalSync>,
    ) {
        match (
            &self.config.http_mode[..],
            self.config.tls_opts.is_some(),
            self.config.static_files.is_some(),
        ) {
            ("auto", false, false) => serve_st(
                &self.config,
                py,
                event_loop,
                signal,
                crate::workers::WorkerCTXBase::new(callback),
                crate::workers::WorkerAcceptorPlain {},
                crate::workers::WorkerHA {
                    opts_h1: self.config.http1_opts.clone(),
                    opts_h2: self.config.http2_opts.clone(),
                },
                std::sync::Arc::new(handle),
            ),
            ("auto", false, true) => serve_st(
                &self.config,
                py,
                event_loop,
                signal,
                crate::workers::WorkerCTXFiles::new(callback, self.config.static_files.clone()),
                crate::workers::WorkerAcceptorPlain {},
                crate::workers::WorkerHA {
                    opts_h1: self.config.http1_opts.clone(),
                    opts_h2: self.config.http2_opts.clone(),
                },
                std::sync::Arc::new(handle),
            ),
            ("auto", true, false) => serve_st(
                &self.config,
                py,
                event_loop,
                signal,
                crate::workers::WorkerCTXBase::new(callback),
                crate::workers::WorkerAcceptorTls {
                    opts: self.config.tls_cfg().into(),
                },
                crate::workers::WorkerHA {
                    opts_h1: self.config.http1_opts.clone(),
                    opts_h2: self.config.http2_opts.clone(),
                },
                std::sync::Arc::new(handle),
            ),
            ("auto", true, true) => serve_st(
                &self.config,
                py,
                event_loop,
                signal,
                crate::workers::WorkerCTXFiles::new(callback, self.config.static_files.clone()),
                crate::workers::WorkerAcceptorTls {
                    opts: self.config.tls_cfg().into(),
                },
                crate::workers::WorkerHA {
                    opts_h1: self.config.http1_opts.clone(),
                    opts_h2: self.config.http2_opts.clone(),
                },
                std::sync::Arc::new(handle),
            ),
            ("1", false, false) => serve_st(
                &self.config,
                py,
                event_loop,
                signal,
                crate::workers::WorkerCTXBase::new(callback),
                crate::workers::WorkerAcceptorPlain {},
                crate::workers::WorkerH1 {
                    opts: self.config.http1_opts.clone(),
                },
                std::sync::Arc::new(handle),
            ),
            ("1", false, true) => serve_st(
                &self.config,
                py,
                event_loop,
                signal,
                crate::workers::WorkerCTXFiles::new(callback, self.config.static_files.clone()),
                crate::workers::WorkerAcceptorPlain {},
                crate::workers::WorkerH1 {
                    opts: self.config.http1_opts.clone(),
                },
                std::sync::Arc::new(handle),
            ),
            ("1", true, false) => serve_st(
                &self.config,
                py,
                event_loop,
                signal,
                crate::workers::WorkerCTXBase::new(callback),
                crate::workers::WorkerAcceptorTls {
                    opts: self.config.tls_cfg().into(),
                },
                crate::workers::WorkerH1 {
                    opts: self.config.http1_opts.clone(),
                },
                std::sync::Arc::new(handle),
            ),
            ("1", true, true) => serve_st(
                &self.config,
                py,
                event_loop,
                signal,
                crate::workers::WorkerCTXFiles::new(callback, self.config.static_files.clone()),
                crate::workers::WorkerAcceptorTls {
                    opts: self.config.tls_cfg().into(),
                },
                crate::workers::WorkerH1 {
                    opts: self.config.http1_opts.clone(),
                },
                std::sync::Arc::new(handle),
            ),
            ("2", false, false) => serve_st(
                &self.config,
                py,
                event_loop,
                signal,
                crate::workers::WorkerCTXBase::new(callback),
                crate::workers::WorkerAcceptorPlain {},
                crate::workers::WorkerH2 {
                    opts: self.config.http2_opts.clone(),
                },
                std::sync::Arc::new(handle),
            ),
            ("2", false, true) => serve_st(
                &self.config,
                py,
                event_loop,
                signal,
                crate::workers::WorkerCTXFiles::new(callback, self.config.static_files.clone()),
                crate::workers::WorkerAcceptorPlain {},
                crate::workers::WorkerH2 {
                    opts: self.config.http2_opts.clone(),
                },
                std::sync::Arc::new(handle),
            ),
            ("2", true, false) => serve_st(
                &self.config,
                py,
                event_loop,
                signal,
                crate::workers::WorkerCTXBase::new(callback),
                crate::workers::WorkerAcceptorTls {
                    opts: self.config.tls_cfg().into(),
                },
                crate::workers::WorkerH2 {
                    opts: self.config.http2_opts.clone(),
                },
                std::sync::Arc::new(handle),
            ),
            ("2", true, true) => serve_st(
                &self.config,
                py,
                event_loop,
                signal,
                crate::workers::WorkerCTXFiles::new(callback, self.config.static_files.clone()),
                crate::workers::WorkerAcceptorTls {
                    opts: self.config.tls_cfg().into(),
                },
                crate::workers::WorkerH2 {
                    opts: self.config.http2_opts.clone(),
                },
                std::sync::Arc::new(handle),
            ),
            _ => unreachable!(),
        }
    }
}

pub(crate) fn serve_mt<C, A, H, F, Ret>(
    cfg: &WorkerConfig,
    py: Python,
    event_loop: &Bound<PyAny>,
    signal: Py<WorkerSignalSync>,
    ctx: C,
    acceptor: A,
    handler: H,
    target: std::sync::Arc<F>,
) where
    F: Fn(
        crate::runtime::RuntimeRef,
        std::sync::Arc<tokio::sync::Notify>,
        crate::callbacks::ArcCBScheduler,
        std::net::SocketAddr,
        std::net::SocketAddr,
        crate::http::HTTPRequest,
        crate::http::HTTPProto,
    ) -> Ret,
    Ret: Future<Output = crate::http::HTTPResponse>,
    Worker<C, A, H, WorkerExcSend, F>: WorkerAcceptor<std::net::TcpListener> + Clone + Send + 'static,
{
    _ = pyo3_log::try_init();

    let worker_id = cfg.id;
    log::info!("Started worker-{worker_id}");

    let tcp_listener = cfg.tcp_listener();
    let backpressure = cfg.backpressure;

    let rtpyloop = std::sync::Arc::new(event_loop.clone().unbind());
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

    let wexec = crate::workers::WorkerExcSend {};
    let wrk = crate::workers::Worker::new(ctx, acceptor, handler, wexec, rth, target);
    let (stx, srx) = tokio::sync::watch::channel(false);

    let main_loop: JoinHandle<anyhow::Result<()>> = rt.inner.spawn(async move {
        wrk.clone().listen(srx, tcp_listener, backpressure).await;

        log::info!("Stopping worker-{worker_id}");

        wrk.tasks.close();
        wrk.tasks.wait().await;

        Python::with_gil(|_| drop(wrk));
        Ok(())
    });

    let pysig = signal.clone_ref(py);
    std::thread::spawn(move || {
        let pyrx = pysig.get().rx.lock().unwrap().take().unwrap();
        _ = pyrx.recv();
        stx.send(true).unwrap();

        while !main_loop.is_finished() {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        Python::with_gil(|py| {
            _ = pysig.get().release(py);
            drop(pysig);
        });
    });

    _ = signal.get().qs.call_method0(py, pyo3::intern!(py, "wait"));
}

pub(crate) fn serve_st<C, A, H, F, Ret>(
    cfg: &WorkerConfig,
    py: Python,
    event_loop: &Bound<PyAny>,
    signal: Py<WorkerSignalSync>,
    ctx: C,
    acceptor: A,
    handler: H,
    target: std::sync::Arc<F>,
) where
    F: Fn(
            crate::runtime::RuntimeRef,
            std::sync::Arc<tokio::sync::Notify>,
            crate::callbacks::ArcCBScheduler,
            std::net::SocketAddr,
            std::net::SocketAddr,
            crate::http::HTTPRequest,
            crate::http::HTTPProto,
        ) -> Ret
        + Send
        + Sync,
    Ret: Future<Output = crate::http::HTTPResponse>,
    C: Clone + Send + 'static,
    A: Clone + Send + 'static,
    H: Clone + Send + 'static,
    Worker<C, A, H, WorkerExcLocal, F>: WorkerAcceptor<std::net::TcpListener> + Clone + Send + 'static,
{
    _ = pyo3_log::try_init();

    let worker_id = cfg.id;
    log::info!("Started worker-{worker_id}");

    let (stx, srx) = tokio::sync::watch::channel(false);
    let mut workers = vec![];

    let py_loop = std::sync::Arc::new(event_loop.clone().unbind());

    for thread_id in 0..cfg.threads {
        log::info!("Started worker-{} runtime-{}", worker_id, thread_id + 1);

        let tcp_listener = cfg.tcp_listener();
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
            let rt = crate::runtime::init_runtime_st(blocking_threads, py_threads, py_threads_idle_timeout, py_loop);
            let rth = rt.handler();
            let wexec = crate::workers::WorkerExcLocal {};
            let wrk = crate::workers::Worker::new(ctx, acceptor, handler, wexec, rth, target);
            let local = tokio::task::LocalSet::new();

            crate::runtime::block_on_local(&rt, local, async move {
                wrk.clone().listen(srx, tcp_listener, backpressure).await;

                log::info!("Stopping worker-{} runtime-{}", worker_id, thread_id + 1);

                wrk.tasks.close();
                wrk.tasks.wait().await;

                Python::with_gil(|_| drop(wrk));
            });

            Python::with_gil(|_| drop(rt));
        }));
    }

    let pysig = signal.clone_ref(py);
    std::thread::spawn(move || {
        let pyrx = pysig.get().rx.lock().unwrap().take().unwrap();
        _ = pyrx.recv();
        stx.send(true).unwrap();
        log::info!("Stopping worker-{worker_id}");
        while let Some(worker) = workers.pop() {
            worker.join().unwrap();
        }

        Python::with_gil(|py| {
            _ = pysig.get().release(py);
            drop(pysig);
        });
    });

    _ = signal.get().qs.call_method0(py, pyo3::intern!(py, "wait"));
}
