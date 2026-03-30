use pyo3::prelude::*;
use std::{future::Future, sync::Arc};
use tokio::{
    runtime::Builder as RuntimeBuilder,
    task::{JoinHandle, LocalSet},
};

use super::{blocking, metrics};

pub trait JoinError {
    #[allow(dead_code)]
    fn is_panic(&self) -> bool;
}

pub trait Runtime: Send + 'static {
    type JoinError: JoinError + Send;
    type JoinHandle: Future<Output = Result<(), Self::JoinError>> + Send;

    fn spawn<F>(&self, fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + Send + 'static;

    fn spawn_blocking<F>(&self, task: F)
    where
        F: FnOnce(Python) + Send + 'static;

    fn spawn_blocking_loopback<F>(&self, task: F)
    where
        F: FnOnce(Python) + Send + 'static;

    fn spawn_cancellable<F>(&self, on_cancel: Arc<tokio::sync::Notify>, fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + Send + 'static;
}

pub trait ContextExt: Runtime {
    fn py_event_loop(&self, py: Python) -> Py<PyAny>;
}

pub(crate) struct RuntimeWrapper {
    pub inner: tokio::runtime::Runtime,
    br: Arc<blocking::BlockingRunner>,
    brlb: Arc<blocking::BlockingRunner>,
    pr: Arc<Py<PyAny>>,
    sig: Arc<tokio::sync::Notify>,
}

impl RuntimeWrapper {
    pub fn new(
        blocking_threads: usize,
        py_threads: usize,
        py_threads_idle_timeout: u64,
        py_loop: Arc<Py<PyAny>>,
        loopback: bool,
        metrics: Option<metrics::ArcWorkerMetrics>,
    ) -> Self {
        let br: Arc<blocking::BlockingRunner> = match metrics {
            Some(metrics) => blocking::BlockingRunner::new_with_metrics(py_threads, py_threads_idle_timeout, metrics),
            None => blocking::BlockingRunner::new(py_threads, py_threads_idle_timeout),
        }
        .into();
        let brlb = match loopback {
            true => Arc::new(blocking::BlockingRunner::new(1, 0)),
            false => br.clone(),
        };

        Self {
            inner: default_runtime(blocking_threads),
            br,
            brlb,
            pr: py_loop,
            sig: tokio::sync::Notify::new().into(),
        }
    }

    pub fn with_runtime(
        rt: tokio::runtime::Runtime,
        py_threads: usize,
        py_threads_idle_timeout: u64,
        py_loop: Arc<Py<PyAny>>,
        loopback: bool,
        metrics: Option<metrics::ArcWorkerMetrics>,
    ) -> Self {
        let br: Arc<blocking::BlockingRunner> = match metrics {
            Some(metrics) => blocking::BlockingRunner::new_with_metrics(py_threads, py_threads_idle_timeout, metrics),
            None => blocking::BlockingRunner::new(py_threads, py_threads_idle_timeout),
        }
        .into();
        let brlb = match loopback {
            true => Arc::new(blocking::BlockingRunner::new(1, 0)),
            false => br.clone(),
        };

        Self {
            inner: rt,
            br,
            brlb,
            pr: py_loop,
            sig: tokio::sync::Notify::new().into(),
        }
    }

    pub fn handler(&self) -> RuntimeRef {
        RuntimeRef::new(
            self.inner.handle().clone(),
            self.br.clone(),
            self.brlb.clone(),
            self.pr.clone(),
            self.sig.clone(),
        )
    }
}

#[derive(Clone)]
pub struct RuntimeRef {
    pub inner: tokio::runtime::Handle,
    innerb: Arc<blocking::BlockingRunner>,
    innerbl: Arc<blocking::BlockingRunner>,
    innerp: Arc<Py<PyAny>>,
    sig: Arc<tokio::sync::Notify>,
}

impl RuntimeRef {
    pub fn new(
        rt: tokio::runtime::Handle,
        br: Arc<blocking::BlockingRunner>,
        brlb: Arc<blocking::BlockingRunner>,
        pyloop: Arc<Py<PyAny>>,
        sig: Arc<tokio::sync::Notify>,
    ) -> Self {
        Self {
            inner: rt,
            innerb: br,
            innerbl: brlb,
            innerp: pyloop,
            sig,
        }
    }

    pub fn close(&self) {
        self.sig.notify_waiters();
    }
}

impl JoinError for tokio::task::JoinError {
    fn is_panic(&self) -> bool {
        tokio::task::JoinError::is_panic(self)
    }
}

impl Runtime for RuntimeRef {
    type JoinError = tokio::task::JoinError;
    type JoinHandle = JoinHandle<()>;

    fn spawn<F>(&self, fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.inner.spawn(fut)
    }

    #[inline]
    fn spawn_blocking<F>(&self, task: F)
    where
        F: FnOnce(Python) + Send + 'static,
    {
        _ = self.innerb.run(task);
    }

    #[inline]
    fn spawn_blocking_loopback<F>(&self, task: F)
    where
        F: FnOnce(Python) + Send + 'static,
    {
        _ = self.innerbl.run(task);
    }

    fn spawn_cancellable<F>(&self, on_cancel: Arc<tokio::sync::Notify>, fut: F) -> Self::JoinHandle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let sig = self.sig.clone();

        self.inner.spawn(async move {
            tokio::select! {
                biased;
                () = fut => {},
                () = sig.notified() => {
                    on_cancel.notify_one();
                }
            };
        })
    }
}

impl ContextExt for RuntimeRef {
    fn py_event_loop(&self, py: Python) -> Py<PyAny> {
        self.innerp.clone_ref(py)
    }
}

fn default_runtime(blocking_threads: usize) -> tokio::runtime::Runtime {
    RuntimeBuilder::new_current_thread()
        .max_blocking_threads(blocking_threads)
        .enable_all()
        .build()
        .unwrap()
}

pub(crate) fn init_runtime_mt(
    threads: usize,
    blocking_threads: usize,
    py_threads: usize,
    py_threads_idle_timeout: u64,
    py_loop: Arc<Py<PyAny>>,
    loopback: bool,
    metrics: Option<metrics::ArcWorkerMetrics>,
) -> RuntimeWrapper {
    RuntimeWrapper::with_runtime(
        RuntimeBuilder::new_multi_thread()
            .worker_threads(threads)
            .max_blocking_threads(blocking_threads)
            .enable_all()
            .build()
            .unwrap(),
        py_threads,
        py_threads_idle_timeout,
        py_loop,
        loopback,
        metrics,
    )
}

pub(crate) fn init_runtime_st(
    blocking_threads: usize,
    py_threads: usize,
    py_threads_idle_timeout: u64,
    py_loop: Arc<Py<PyAny>>,
    loopback: bool,
    metrics: Option<metrics::ArcWorkerMetrics>,
) -> RuntimeWrapper {
    RuntimeWrapper::new(
        blocking_threads,
        py_threads,
        py_threads_idle_timeout,
        py_loop,
        loopback,
        metrics,
    )
}

pub(crate) fn block_on_local<F>(rt: &RuntimeWrapper, local: LocalSet, fut: F)
where
    F: Future + 'static,
{
    local.block_on(&rt.inner, fut);
}
