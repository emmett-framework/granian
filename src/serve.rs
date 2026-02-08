use pyo3::prelude::*;
use std::sync::Arc;

use super::workers::{Worker, WorkerAcceptor, WorkerConfig, WorkerSignal};

macro_rules! serve_fn {
    (mt $name:ident, $listener:ty, $listener_gen:ident) => {
        pub(crate) fn $name<C, A, H, F, M, Ret>(
            cfg: &WorkerConfig,
            py: Python,
            event_loop: &Bound<PyAny>,
            signal: Py<WorkerSignal>,
            metrics: (M, Option<crate::metrics::ArcWorkerMetrics>),
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
            M: Clone + Sync,
            Ret: Future<Output = crate::http::HTTPResponse>,
            Worker<C, A, H, F, M>: WorkerAcceptor<$listener> + Send + 'static,
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
                    metrics.1.clone(),
                )
            });
            let rth = rt.handler();
            let srx = signal.get().rx.lock().unwrap().take().unwrap();
            let mc_notify = Arc::new(tokio::sync::Notify::new());

            if let Some(metrics_interval) = cfg.metrics.0 {
                #[cfg(not(Py_GIL_DISABLED))]
                crate::metrics::spawn_ipc_collector(
                    rth.clone(),
                    srx.clone(),
                    mc_notify.clone(),
                    metrics.1.clone().unwrap(),
                    metrics_interval,
                    cfg.ipc.as_ref().unwrap().clone_ref(py),
                );
                #[cfg(Py_GIL_DISABLED)]
                crate::metrics::spawn_local_collector(
                    rth.clone(),
                    srx.clone(),
                    mc_notify.clone(),
                    metrics.1.clone().unwrap(),
                    metrics_interval,
                    (cfg.id - 1).try_into().unwrap(),
                    cfg.metrics.1.as_ref().unwrap().clone_ref(py),
                );
            } else {
                mc_notify.notify_one();
            }

            let wrk = crate::workers::Worker::new(ctx, acceptor, handler, rth, target, metrics.0);
            let tasks = wrk.tasks.clone();

            let main_loop = crate::runtime::run_until_complete(&rt, event_loop.clone(), async move {
                wrk.listen(srx, listener, backpressure).await;

                log::info!("Stopping worker-{worker_id}");

                wrk.rt.close();
                tasks.close();
                tasks.wait().await;
                mc_notify.notified().await;

                Python::attach(|_| drop(wrk));
                Ok(())
            });

            drop(rt);

            if let Err(err) = main_loop {
                log::error!("{err}");
                std::process::exit(1);
            }
        }
    };

    (st $name:ident, $listener:ty, $listener_gen:ident) => {
        pub(crate) fn $name<C, A, H, F, M, Ret>(
            cfg: &WorkerConfig,
            _py: (),
            event_loop: &Bound<PyAny>,
            signal: Py<WorkerSignal>,
            metrics: (M, Option<crate::metrics::ArcWorkerMetrics>),
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
            M: Clone + Send,
            Worker<C, A, H, F, M>: WorkerAcceptor<$listener> + Send + 'static,
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
                let metrics = metrics.clone();
                let ctx = ctx.clone();
                let acceptor = acceptor.clone();
                let handler = handler.clone();
                let target = target.clone();
                let py_loop = py_loop.clone();
                let srx = srx.clone();

                workers.push(std::thread::spawn(move || {
                    let rt = crate::runtime::init_runtime_st(
                        blocking_threads,
                        py_threads,
                        py_threads_idle_timeout,
                        py_loop,
                        metrics.1.clone(),
                    );
                    let rth = rt.handler();
                    let wrk = crate::workers::Worker::new(ctx, acceptor, handler, rth, target, metrics.0);
                    let local = tokio::task::LocalSet::new();
                    let tasks = wrk.tasks.clone();

                    crate::runtime::block_on_local(&rt, local, async move {
                        wrk.listen(srx, listener, backpressure).await;

                        log::info!("Stopping worker-{} runtime-{}", worker_id, thread_id + 1);

                        wrk.rt.close();
                        tasks.close();
                        tasks.wait().await;

                        Python::attach(|_| drop(wrk));
                    });

                    Python::attach(|_| drop(rt));
                }));
            }

            let rtm = crate::runtime::init_runtime_mt(1, 1, 0, 0, Arc::new(event_loop.clone().unbind()), None);
            let mut pyrx = signal.get().rx.lock().unwrap().take().unwrap();
            let mc_notify = Arc::new(tokio::sync::Notify::new());

            if let Some(metrics_interval) = cfg.metrics.0 {
                #[cfg(not(Py_GIL_DISABLED))]
                crate::metrics::spawn_ipc_collector(
                    rtm.handler(),
                    pyrx.clone(),
                    mc_notify.clone(),
                    metrics.1.clone().unwrap(),
                    metrics_interval,
                    cfg.ipc.as_ref().unwrap().clone_ref(event_loop.py()),
                );
                #[cfg(Py_GIL_DISABLED)]
                crate::metrics::spawn_local_collector(
                    rtm.handler(),
                    pyrx.clone(),
                    mc_notify.clone(),
                    metrics.1.clone().unwrap(),
                    metrics_interval,
                    (cfg.id - 1).try_into().unwrap(),
                    cfg.metrics.1.as_ref().unwrap().clone_ref(event_loop.py()),
                );
            } else {
                mc_notify.notify_one();
            }

            let main_loop = crate::runtime::run_until_complete(&rtm, event_loop.clone(), async move {
                let _ = pyrx.changed().await;
                stx.send(true).unwrap();
                log::info!("Stopping worker-{worker_id}");
                while let Some(worker) = workers.pop() {
                    worker.join().unwrap();
                }
                mc_notify.notified().await;
                Ok(())
            });

            drop(rtm);

            if let Err(err) = main_loop {
                log::error!("{err}");
                std::process::exit(1);
            }
        }
    };

    (fut $name:ident, $listener:ty, $listener_gen:ident) => {
        pub(crate) fn $name<'p, C, A, H, F, M, Ret>(
            cfg: &WorkerConfig,
            _py: (),
            event_loop: &Bound<'p, PyAny>,
            signal: Py<WorkerSignal>,
            metrics: (M, Option<crate::metrics::ArcWorkerMetrics>),
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
            M: Clone + Send,
            Worker<C, A, H, F, M>: WorkerAcceptor<$listener> + Send + 'static,
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
                let rt = crate::runtime::init_runtime_st(
                    blocking_threads,
                    py_threads,
                    py_threads_idle_timeout,
                    pyloop_r1,
                    metrics.1.clone(),
                );
                let rth = rt.handler();
                let wrk = crate::workers::Worker::new(ctx, acceptor, handler, rth, target, metrics.0);
                let tasks = wrk.tasks.clone();

                rt.inner.block_on(async move {
                    wrk.listen(srx, tcp_listener, backpressure).await;

                    log::info!("Stopping worker-{worker_id}");

                    wrk.rt.close();
                    tasks.close();
                    tasks.wait().await;

                    Python::attach(|_| drop(wrk));
                });

                Python::attach(|_| drop(rt));
            });

            let ret = event_loop.call_method0("create_future").unwrap();
            let pyfut = ret.clone().unbind();

            std::thread::spawn(move || {
                let rt = crate::runtime::init_runtime_st(1, 0, 0, pyloop_r2.clone(), None);
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

macro_rules! gen_serve_impl {
    ($sm:expr, $self:expr, $py:expr, $event_loop:expr, $signal:expr, $metrics:expr, $metrics_opt:expr, $ctx:expr, $acceptor:expr, $proto:expr, $target:expr) => {{
        $sm(
            &$self.config,
            $py,
            $event_loop,
            $signal,
            ($metrics.clone(), $metrics_opt),
            $ctx,
            $acceptor,
            $proto,
            $target,
        )
    }};
}

macro_rules! gen_serve_match_proto {
    ($sm:expr, $self:expr, $py:expr, $event_loop:expr, $signal:expr, $metrics:expr, $metrics_opt:expr, $ctx:expr, $acceptor:expr, $target:expr, $targetws:expr) => {{
        match (&$self.config.http_mode[..], $self.config.websockets_enabled) {
            ("auto", false) => crate::serve::gen_serve_impl!(
                $sm,
                $self,
                $py,
                $event_loop,
                $signal,
                $metrics,
                $metrics_opt,
                $ctx,
                $acceptor,
                crate::workers::WorkerHandlerHA {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
                    metrics: $metrics.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnNoUpgrades>,
                },
                $target
            ),
            ("auto", true) => crate::serve::gen_serve_impl!(
                $sm,
                $self,
                $py,
                $event_loop,
                $signal,
                $metrics,
                $metrics_opt,
                $ctx,
                $acceptor,
                crate::workers::WorkerHandlerHA {
                    opts_h1: $self.config.http1_opts.clone(),
                    opts_h2: $self.config.http2_opts.clone(),
                    metrics: $metrics.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnUpgrades>,
                },
                $targetws
            ),
            ("1", false) => crate::serve::gen_serve_impl!(
                $sm,
                $self,
                $py,
                $event_loop,
                $signal,
                $metrics,
                $metrics_opt,
                $ctx,
                $acceptor,
                crate::workers::WorkerHandlerH1 {
                    opts: $self.config.http1_opts.clone(),
                    metrics: $metrics.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnNoUpgrades>,
                },
                $target
            ),
            ("1", true) => crate::serve::gen_serve_impl!(
                $sm,
                $self,
                $py,
                $event_loop,
                $signal,
                $metrics,
                $metrics_opt,
                $ctx,
                $acceptor,
                crate::workers::WorkerHandlerH1 {
                    opts: $self.config.http1_opts.clone(),
                    metrics: $metrics.clone(),
                    _upgrades: std::marker::PhantomData::<crate::workers::WorkerMarkerConnUpgrades>,
                },
                $targetws
            ),
            ("2", _) => crate::serve::gen_serve_impl!(
                $sm,
                $self,
                $py,
                $event_loop,
                $signal,
                $metrics,
                $metrics_opt,
                $ctx,
                $acceptor,
                crate::workers::WorkerHandlerH2 {
                    opts: $self.config.http2_opts.clone(),
                    metrics: $metrics.clone(),
                },
                $target
            ),
            _ => unreachable!(),
        }
    }};
}

macro_rules! gen_serve_match_tls {
    ($sm:expr, $self:expr, $py:expr, $event_loop:expr, $signal:expr, $metrics:expr, $metrics_opt:expr, $ctx:expr, $acceptor_plain:ident, $acceptor_tls:ident, $target:expr, $targetws:expr) => {{
        match $self.config.tls_opts.is_some() {
            false => crate::serve::gen_serve_match_proto!(
                $sm,
                $self,
                $py,
                $event_loop,
                $signal,
                $metrics,
                $metrics_opt,
                $ctx,
                crate::workers::$acceptor_plain {},
                $target,
                $targetws
            ),
            true => crate::serve::gen_serve_match_proto!(
                $sm,
                $self,
                $py,
                $event_loop,
                $signal,
                $metrics,
                $metrics_opt,
                $ctx,
                crate::workers::$acceptor_tls {
                    opts: $self.config.tls_cfg().into(),
                },
                $target,
                $targetws
            ),
        }
    }};
}

macro_rules! gen_serve_match_files {
    ($sm:expr, $self:expr, $py:expr, $event_loop:expr, $signal:expr, $metrics:expr, $metrics_opt:expr, $callback:expr, $acceptor_plain:ident, $acceptor_tls:ident, $target:expr, $targetws:expr) => {{
        match $self.config.static_files.is_some() {
            false => crate::serve::gen_serve_match_tls!(
                $sm,
                $self,
                $py,
                $event_loop,
                $signal,
                $metrics,
                $metrics_opt,
                crate::workers::WorkerCTXBase::new($callback, $metrics.clone()),
                $acceptor_plain,
                $acceptor_tls,
                $target,
                $targetws
            ),
            true => crate::serve::gen_serve_match_tls!(
                $sm,
                $self,
                $py,
                $event_loop,
                $signal,
                $metrics,
                $metrics_opt,
                crate::workers::WorkerCTXFiles::new($callback, $metrics.clone(), $self.config.static_files.clone()),
                $acceptor_plain,
                $acceptor_tls,
                $target,
                $targetws
            ),
        }
    }};
}

macro_rules! gen_serve_match {
    ($sm:expr, $acceptor_plain:ident, $acceptor_tls:ident, $self:expr, $py:expr, $callback:expr, $event_loop:expr, $signal:expr, $target:expr, $targetws:expr) => {{
        let metrics_obj = std::sync::Arc::new(crate::metrics::WorkerMetrics::new());
        match $self.config.metrics.0.is_some() {
            false => crate::serve::gen_serve_match_files!(
                $sm,
                $self,
                $py,
                $event_loop,
                $signal,
                (),
                None,
                $callback,
                $acceptor_plain,
                $acceptor_tls,
                $target,
                $targetws
            ),
            true => crate::serve::gen_serve_match_files!(
                $sm,
                $self,
                $py,
                $event_loop,
                $signal,
                metrics_obj.clone(),
                Some(metrics_obj.clone()),
                $callback,
                $acceptor_plain,
                $acceptor_tls,
                $target,
                $targetws
            ),
        }
    }};
}

pub(crate) use gen_serve_impl;
pub(crate) use gen_serve_match;
pub(crate) use gen_serve_match_files;
pub(crate) use gen_serve_match_proto;
pub(crate) use gen_serve_match_tls;
