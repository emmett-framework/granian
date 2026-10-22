use pyo3::prelude::*;
use std::{
    sync::{Arc, atomic},
    thread, time,
};

use super::{
    metrics,
    scheduler::{FlowRef, PopError, PushError, Scheduler, SchedulerImpl},
};

pub(crate) struct BlockingTask {
    inner: Box<dyn FnOnce(Python) + Send + 'static>,
}

impl BlockingTask {
    #[inline]
    pub fn new<T>(inner: T) -> BlockingTask
    where
        T: FnOnce(Python) + Send + 'static,
    {
        Self { inner: Box::new(inner) }
    }

    #[inline(always)]
    pub fn run(self, py: Python) {
        (self.inner)(py);
    }
}

pub(crate) enum BlockingRunner {
    Empty,
    Mono(BlockingRunnerMono<()>),
    Pool(BlockingRunnerPool<()>),
    MetricsMono(BlockingRunnerMono<metrics::ArcWorkerMetrics>),
    MetricsPool(BlockingRunnerPool<metrics::ArcWorkerMetrics>),
}

impl BlockingRunner {
    pub fn new(max_threads: usize, idle_timeout: u64, scheduler: Option<Arc<SchedulerImpl>>) -> Self {
        let scheduler = scheduler.unwrap_or_else(default_scheduler);
        match max_threads {
            0 => Self::Empty,
            1 => Self::Mono(BlockingRunnerMono::<()>::new(scheduler)),
            _ => Self::Pool(BlockingRunnerPool::<()>::new(max_threads, idle_timeout, scheduler)),
        }
    }

    pub fn new_with_metrics(
        max_threads: usize,
        idle_timeout: u64,
        metrics: metrics::ArcWorkerMetrics,
        scheduler: Option<Arc<SchedulerImpl>>,
    ) -> Self {
        let scheduler = scheduler.unwrap_or_else(default_scheduler);
        match max_threads {
            0 => Self::Empty,
            1 => Self::MetricsMono(BlockingRunnerMono::<metrics::ArcWorkerMetrics>::new(scheduler, metrics)),
            _ => Self::MetricsPool(BlockingRunnerPool::<metrics::ArcWorkerMetrics>::new(
                max_threads,
                idle_timeout,
                scheduler,
                metrics,
            )),
        }
    }

    #[inline]
    pub fn run<T>(&self, task: T) -> Result<(), PushError>
    where
        T: FnOnce(Python) + Send + 'static,
    {
        self.run_with_flow(task, None)
    }

    #[inline]
    pub fn run_with_flow<T>(&self, task: T, flow: Option<&FlowRef>) -> Result<(), PushError>
    where
        T: FnOnce(Python) + Send + 'static,
    {
        match self {
            Self::Mono(runner) => runner.run(task, flow),
            Self::Pool(runner) => runner.run(task, flow),
            Self::MetricsMono(runner) => runner.run(task, flow),
            Self::MetricsPool(runner) => runner.run(task, flow),
            Self::Empty => Ok(()),
        }
    }
}

fn default_scheduler() -> Arc<SchedulerImpl> {
    Arc::new(SchedulerImpl::fifo())
}

pub(crate) struct BlockingRunnerMono<M> {
    sched: Arc<SchedulerImpl>,
    metrics: M,
}

impl BlockingRunnerMono<()> {
    pub fn new(sched: Arc<SchedulerImpl>) -> Self {
        let ret = Self {
            sched: sched.clone(),
            metrics: (),
        };
        thread::spawn(move || blocking_worker(sched));

        ret
    }

    #[inline]
    pub fn run<T>(&self, task: T, flow: Option<&FlowRef>) -> Result<(), PushError>
    where
        T: FnOnce(Python) + Send + 'static,
    {
        self.sched.push(BlockingTask::new(task), flow)
    }
}

impl BlockingRunnerMono<metrics::ArcWorkerMetrics> {
    pub fn new(sched: Arc<SchedulerImpl>, metrics: metrics::ArcWorkerMetrics) -> Self {
        let ret = Self {
            sched: sched.clone(),
            metrics: metrics.clone(),
        };
        thread::spawn(move || blocking_worker_with_metrics(sched, metrics));
        ret.metrics.blocking_threads.store(1, atomic::Ordering::Relaxed);

        ret
    }

    #[inline]
    pub fn run<T>(&self, task: T, flow: Option<&FlowRef>) -> Result<(), PushError>
    where
        T: FnOnce(Python) + Send + 'static,
    {
        self.sched.push(BlockingTask::new(task), flow).map(|()| {
            self.metrics.blocking_queue.fetch_add(1, atomic::Ordering::Relaxed);
        })
    }
}

pub(crate) struct BlockingRunnerPool<M> {
    sched: Arc<SchedulerImpl>,
    threads: Arc<atomic::AtomicUsize>,
    tmax: usize,
    idle: Arc<atomic::AtomicUsize>,
    idle_timeout: time::Duration,
    metrics: M,
}

impl BlockingRunnerPool<()> {
    pub fn new(max_threads: usize, idle_timeout: u64, sched: Arc<SchedulerImpl>) -> Self {
        let idle = Arc::new(atomic::AtomicUsize::new(0));
        let ret = Self {
            sched: sched.clone(),
            threads: Arc::new(1.into()),
            tmax: max_threads,
            idle: idle.clone(),
            idle_timeout: time::Duration::from_secs(idle_timeout),
            metrics: (),
        };

        // always spawn the first thread
        thread::spawn(move || blocking_worker_idle(sched, idle));

        ret
    }

    #[inline(always)]
    fn spawn_thread(&self, current_count: usize) {
        if self
            .threads
            .compare_exchange(
                current_count,
                current_count + 1,
                atomic::Ordering::Relaxed,
                atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            return;
        }

        let sched = self.sched.clone();
        let tcount = self.threads.clone();
        let idle = self.idle.clone();
        let timeout = self.idle_timeout;

        thread::spawn(move || {
            blocking_worker_timeout(sched, timeout, idle);
            tcount.fetch_sub(1, atomic::Ordering::Relaxed);
        });
    }

    #[inline]
    pub fn run<T>(&self, task: T, flow: Option<&FlowRef>) -> Result<(), PushError>
    where
        T: FnOnce(Python) + Send + 'static,
    {
        let threads = self.threads.load(atomic::Ordering::Relaxed).cast_signed();
        self.sched.push(BlockingTask::new(task), flow)?;
        let idle = self.idle.load(atomic::Ordering::Relaxed).cast_signed();
        let overload = self.sched.len().cast_signed() - idle;
        if (overload > 0) && (threads < self.tmax.cast_signed()) {
            self.spawn_thread(threads.cast_unsigned());
        }
        Ok(())
    }
}

impl BlockingRunnerPool<metrics::ArcWorkerMetrics> {
    pub fn new(
        max_threads: usize,
        idle_timeout: u64,
        sched: Arc<SchedulerImpl>,
        metrics: metrics::ArcWorkerMetrics,
    ) -> Self {
        let idle = Arc::new(atomic::AtomicUsize::new(0));
        let ret = Self {
            sched: sched.clone(),
            threads: Arc::new(0.into()),
            tmax: max_threads,
            idle: idle.clone(),
            idle_timeout: time::Duration::from_secs(idle_timeout),
            metrics: metrics.clone(),
        };

        // always spawn the first thread
        thread::spawn(move || blocking_worker_idle_with_metrics(sched, idle, metrics));
        ret.metrics.blocking_threads.store(1, atomic::Ordering::Relaxed);

        ret
    }

    #[inline(always)]
    fn spawn_thread(&self, current_count: usize) {
        if self
            .metrics
            .blocking_threads
            .compare_exchange(
                current_count,
                current_count + 1,
                atomic::Ordering::Relaxed,
                atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            return;
        }

        let sched = self.sched.clone();
        let metrics = self.metrics.clone();
        let idle = self.idle.clone();
        let timeout = self.idle_timeout;

        thread::spawn(move || {
            blocking_worker_timeout_with_metrics(sched, timeout, idle, metrics.clone());
            metrics.blocking_threads.fetch_sub(1, atomic::Ordering::Relaxed);
        });
    }

    #[inline]
    pub fn run<T>(&self, task: T, flow: Option<&FlowRef>) -> Result<(), PushError>
    where
        T: FnOnce(Python) + Send + 'static,
    {
        let threads = self
            .metrics
            .blocking_threads
            .load(atomic::Ordering::Relaxed)
            .cast_signed();
        self.sched.push(BlockingTask::new(task), flow)?;
        self.metrics.blocking_queue.fetch_add(1, atomic::Ordering::Relaxed);
        let idle = self.idle.load(atomic::Ordering::Relaxed).cast_signed();
        let overload = self.sched.len().cast_signed() - idle;
        if (overload > 0) && (threads < self.tmax.cast_signed()) {
            self.spawn_thread(threads.cast_unsigned());
        }
        Ok(())
    }
}

fn blocking_worker(sched: Arc<SchedulerImpl>) {
    Python::attach(|py| {
        while let Some(task) = py.detach(|| sched.pop()) {
            task.run(py);
        }
    });
}

fn blocking_worker_idle(sched: Arc<SchedulerImpl>, idle: Arc<atomic::AtomicUsize>) {
    Python::attach(|py| {
        loop {
            let task = py.detach(|| {
                idle.fetch_add(1, atomic::Ordering::Relaxed);
                let task = sched.pop();
                idle.fetch_sub(1, atomic::Ordering::Relaxed);
                task
            });
            match task {
                Some(task) => task.run(py),
                None => break,
            }
        }
    });
}

fn blocking_worker_timeout(sched: Arc<SchedulerImpl>, timeout: time::Duration, idle: Arc<atomic::AtomicUsize>) {
    Python::attach(|py| {
        loop {
            let task = py.detach(|| {
                idle.fetch_add(1, atomic::Ordering::Relaxed);
                let task = sched.pop_timeout(timeout);
                idle.fetch_sub(1, atomic::Ordering::Relaxed);
                task
            });
            match task {
                Ok(task) => task.run(py),
                Err(PopError::Timeout) => break,
                Err(PopError::Closed) => break,
            }
        }
    });
}

#[cfg(not(Py_GIL_DISABLED))]
fn blocking_worker_with_metrics(sched: Arc<SchedulerImpl>, metrics: Arc<metrics::WorkerMetrics>) {
    Python::attach(|py| {
        let mut t_wait = time::Instant::now();
        loop {
            let task = py.detach(|| {
                let t = time::Instant::now();
                let task = sched.pop();
                metrics
                    .blocking_idle_cumul
                    .fetch_add(t.elapsed().as_micros() as usize, atomic::Ordering::Relaxed);
                if task.is_some() {
                    metrics.blocking_queue.fetch_sub(1, atomic::Ordering::Relaxed);
                }
                t_wait = time::Instant::now();
                task
            });
            match task {
                Some(task) => {
                    metrics
                        .py_wait_cumul
                        .fetch_add(t_wait.elapsed().as_micros() as usize, atomic::Ordering::Relaxed);
                    task.run(py);
                    metrics
                        .blocking_busy_cumul
                        .fetch_add(t_wait.elapsed().as_micros() as usize, atomic::Ordering::Relaxed);
                }
                None => break,
            }
        }
    });
}

#[cfg(Py_GIL_DISABLED)]
fn blocking_worker_with_metrics(sched: Arc<SchedulerImpl>, metrics: Arc<metrics::WorkerMetrics>) {
    Python::attach(|py| {
        loop {
            let task = py.detach(|| {
                let t = time::Instant::now();
                let task = sched.pop();
                metrics
                    .blocking_idle_cumul
                    .fetch_add(t.elapsed().as_micros() as usize, atomic::Ordering::Relaxed);
                if task.is_some() {
                    metrics.blocking_queue.fetch_sub(1, atomic::Ordering::Relaxed);
                }
                task
            });
            match task {
                Some(task) => {
                    let t = time::Instant::now();
                    task.run(py);
                    metrics
                        .blocking_busy_cumul
                        .fetch_add(t.elapsed().as_micros() as usize, atomic::Ordering::Relaxed);
                }
                None => break,
            }
        }
    });
}

#[cfg(not(Py_GIL_DISABLED))]
fn blocking_worker_idle_with_metrics(
    sched: Arc<SchedulerImpl>,
    idle: Arc<atomic::AtomicUsize>,
    metrics: Arc<metrics::WorkerMetrics>,
) {
    Python::attach(|py| {
        let mut t_wait = time::Instant::now();
        loop {
            let task = py.detach(|| {
                let t = time::Instant::now();
                idle.fetch_add(1, atomic::Ordering::Relaxed);
                let task = sched.pop();
                idle.fetch_sub(1, atomic::Ordering::Relaxed);
                metrics
                    .blocking_idle_cumul
                    .fetch_add(t.elapsed().as_micros() as usize, atomic::Ordering::Relaxed);
                if task.is_some() {
                    metrics.blocking_queue.fetch_sub(1, atomic::Ordering::Relaxed);
                }
                t_wait = time::Instant::now();
                task
            });
            match task {
                Some(task) => {
                    metrics
                        .py_wait_cumul
                        .fetch_add(t_wait.elapsed().as_micros() as usize, atomic::Ordering::Relaxed);
                    task.run(py);
                    metrics
                        .blocking_busy_cumul
                        .fetch_add(t_wait.elapsed().as_micros() as usize, atomic::Ordering::Relaxed);
                }
                None => break,
            }
        }
    });
}

#[cfg(Py_GIL_DISABLED)]
fn blocking_worker_idle_with_metrics(
    sched: Arc<SchedulerImpl>,
    idle: Arc<atomic::AtomicUsize>,
    metrics: Arc<metrics::WorkerMetrics>,
) {
    Python::attach(|py| {
        loop {
            let task = py.detach(|| {
                let t = time::Instant::now();
                idle.fetch_add(1, atomic::Ordering::Relaxed);
                let task = sched.pop();
                idle.fetch_sub(1, atomic::Ordering::Relaxed);
                metrics
                    .blocking_idle_cumul
                    .fetch_add(t.elapsed().as_micros() as usize, atomic::Ordering::Relaxed);
                if task.is_some() {
                    metrics.blocking_queue.fetch_sub(1, atomic::Ordering::Relaxed);
                }
                task
            });
            match task {
                Some(task) => {
                    let t = time::Instant::now();
                    task.run(py);
                    metrics
                        .blocking_busy_cumul
                        .fetch_add(t.elapsed().as_micros() as usize, atomic::Ordering::Relaxed);
                }
                None => break,
            }
        }
    });
}

#[cfg(not(Py_GIL_DISABLED))]
fn blocking_worker_timeout_with_metrics(
    sched: Arc<SchedulerImpl>,
    timeout: time::Duration,
    idle: Arc<atomic::AtomicUsize>,
    metrics: Arc<metrics::WorkerMetrics>,
) {
    Python::attach(|py| {
        let mut t_wait = time::Instant::now();
        loop {
            let task = py.detach(|| {
                let t = time::Instant::now();
                idle.fetch_add(1, atomic::Ordering::Relaxed);
                let task = sched.pop_timeout(timeout);
                idle.fetch_sub(1, atomic::Ordering::Relaxed);
                metrics
                    .blocking_idle_cumul
                    .fetch_add(t.elapsed().as_micros() as usize, atomic::Ordering::Relaxed);
                if task.is_ok() {
                    metrics.blocking_queue.fetch_sub(1, atomic::Ordering::Relaxed);
                }
                t_wait = time::Instant::now();
                task
            });
            match task {
                Ok(task) => {
                    metrics
                        .py_wait_cumul
                        .fetch_add(t_wait.elapsed().as_micros() as usize, atomic::Ordering::Relaxed);
                    task.run(py);
                    metrics
                        .blocking_busy_cumul
                        .fetch_add(t_wait.elapsed().as_micros() as usize, atomic::Ordering::Relaxed);
                }
                Err(_) => break,
            }
        }
    });
}

#[cfg(Py_GIL_DISABLED)]
fn blocking_worker_timeout_with_metrics(
    sched: Arc<SchedulerImpl>,
    timeout: time::Duration,
    idle: Arc<atomic::AtomicUsize>,
    metrics: Arc<metrics::WorkerMetrics>,
) {
    Python::attach(|py| {
        loop {
            let task = py.detach(|| {
                let t = time::Instant::now();
                idle.fetch_add(1, atomic::Ordering::Relaxed);
                let task = sched.pop_timeout(timeout);
                idle.fetch_sub(1, atomic::Ordering::Relaxed);
                metrics
                    .blocking_idle_cumul
                    .fetch_add(t.elapsed().as_micros() as usize, atomic::Ordering::Relaxed);
                if task.is_ok() {
                    metrics.blocking_queue.fetch_sub(1, atomic::Ordering::Relaxed);
                }
                task
            });
            match task {
                Ok(task) => {
                    let t = time::Instant::now();
                    task.run(py);
                    metrics
                        .blocking_busy_cumul
                        .fetch_add(t.elapsed().as_micros() as usize, atomic::Ordering::Relaxed);
                }
                Err(_) => break,
            }
        }
    });
}
