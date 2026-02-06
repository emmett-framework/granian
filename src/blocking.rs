use crossbeam_channel as channel;
use pyo3::prelude::*;
use std::{
    sync::{Arc, atomic},
    thread, time,
};

use super::metrics;

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
    pub fn new(max_threads: usize, idle_timeout: u64) -> Self {
        match max_threads {
            0 => Self::Empty,
            1 => Self::Mono(BlockingRunnerMono::<()>::new()),
            _ => Self::Pool(BlockingRunnerPool::<()>::new(max_threads, idle_timeout)),
        }
    }

    pub fn new_with_metrics(max_threads: usize, idle_timeout: u64, metrics: metrics::ArcWorkerMetrics) -> Self {
        match max_threads {
            0 => Self::Empty,
            1 => Self::MetricsMono(BlockingRunnerMono::<metrics::ArcWorkerMetrics>::new(metrics)),
            _ => Self::MetricsPool(BlockingRunnerPool::<metrics::ArcWorkerMetrics>::new(
                max_threads,
                idle_timeout,
                metrics,
            )),
        }
    }

    #[inline]
    pub fn run<T>(&self, task: T) -> Result<(), channel::SendError<BlockingTask>>
    where
        T: FnOnce(Python) + Send + 'static,
    {
        match self {
            Self::Mono(runner) => runner.run(task),
            Self::Pool(runner) => runner.run(task),
            Self::MetricsMono(runner) => runner.run(task),
            Self::MetricsPool(runner) => runner.run(task),
            Self::Empty => Ok(()),
        }
    }
}

pub(crate) struct BlockingRunnerMono<M> {
    queue: channel::Sender<BlockingTask>,
    metrics: M,
}

impl BlockingRunnerMono<()> {
    pub fn new() -> Self {
        let (qtx, qrx) = channel::unbounded();
        let ret = Self {
            queue: qtx,
            metrics: (),
        };
        thread::spawn(move || blocking_worker(qrx));

        ret
    }

    #[inline]
    pub fn run<T>(&self, task: T) -> Result<(), channel::SendError<BlockingTask>>
    where
        T: FnOnce(Python) + Send + 'static,
    {
        self.queue.send(BlockingTask::new(task))
    }
}

impl BlockingRunnerMono<metrics::ArcWorkerMetrics> {
    pub fn new(metrics: metrics::ArcWorkerMetrics) -> Self {
        let (qtx, qrx) = channel::unbounded();
        let ret = Self {
            queue: qtx,
            metrics: metrics.clone(),
        };
        thread::spawn(move || blocking_worker_with_metrics(qrx, metrics));
        ret.metrics.blocking_threads.store(1, atomic::Ordering::Release);

        ret
    }

    #[inline]
    pub fn run<T>(&self, task: T) -> Result<(), channel::SendError<BlockingTask>>
    where
        T: FnOnce(Python) + Send + 'static,
    {
        self.queue.send(BlockingTask::new(task)).map(|()| {
            self.metrics.blocking_queue.fetch_add(1, atomic::Ordering::Release);
        })
    }
}

pub(crate) struct BlockingRunnerPool<M> {
    queue: channel::Sender<BlockingTask>,
    tq: channel::Receiver<BlockingTask>,
    threads: Arc<atomic::AtomicUsize>,
    tmax: usize,
    idle_timeout: time::Duration,
    metrics: M,
}

impl BlockingRunnerPool<()> {
    pub fn new(max_threads: usize, idle_timeout: u64) -> Self {
        let (qtx, qrx) = channel::unbounded();
        let ret = Self {
            queue: qtx,
            tq: qrx.clone(),
            threads: Arc::new(1.into()),
            tmax: max_threads,
            idle_timeout: time::Duration::from_secs(idle_timeout),
            metrics: (),
        };

        // always spawn the first thread
        thread::spawn(move || blocking_worker(qrx));

        ret
    }

    #[inline(always)]
    fn spawn_thread(&self, current_count: usize) {
        if self
            .threads
            .compare_exchange(
                current_count,
                current_count + 1,
                atomic::Ordering::Release,
                atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            return;
        }

        let queue = self.tq.clone();
        let tcount = self.threads.clone();
        let timeout = self.idle_timeout;

        thread::spawn(move || {
            blocking_worker_idle(queue, timeout);
            tcount.fetch_sub(1, atomic::Ordering::Release);
        });
    }

    #[inline]
    pub fn run<T>(&self, task: T) -> Result<(), channel::SendError<BlockingTask>>
    where
        T: FnOnce(Python) + Send + 'static,
    {
        let threads = self.threads.load(atomic::Ordering::Acquire).cast_signed();
        self.queue.send(BlockingTask::new(task))?;
        let overload = self.queue.len().cast_signed() - threads;
        if (overload > 0) && (threads < self.tmax.cast_signed()) {
            self.spawn_thread(threads.cast_unsigned());
        }
        Ok(())
    }
}

impl BlockingRunnerPool<metrics::ArcWorkerMetrics> {
    pub fn new(max_threads: usize, idle_timeout: u64, metrics: metrics::ArcWorkerMetrics) -> Self {
        let (qtx, qrx) = channel::unbounded();
        let ret = Self {
            queue: qtx,
            tq: qrx.clone(),
            // NOTE: we use metrics in place of this atomic
            threads: Arc::new(0.into()),
            tmax: max_threads,
            idle_timeout: time::Duration::from_secs(idle_timeout),
            metrics: metrics.clone(),
        };

        // always spawn the first thread
        thread::spawn(move || blocking_worker_with_metrics(qrx, metrics));
        ret.metrics.blocking_threads.store(1, atomic::Ordering::Release);

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
                atomic::Ordering::Release,
                atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            return;
        }

        let queue = self.tq.clone();
        let metrics = self.metrics.clone();
        let timeout = self.idle_timeout;

        thread::spawn(move || {
            blocking_worker_idle_with_metrics(queue, timeout, metrics.clone());
            metrics.blocking_threads.fetch_sub(1, atomic::Ordering::Release);
        });
    }

    #[inline]
    pub fn run<T>(&self, task: T) -> Result<(), channel::SendError<BlockingTask>>
    where
        T: FnOnce(Python) + Send + 'static,
    {
        let threads = self
            .metrics
            .blocking_threads
            .load(atomic::Ordering::Acquire)
            .cast_signed();
        self.queue.send(BlockingTask::new(task))?;
        self.metrics.blocking_queue.fetch_add(1, atomic::Ordering::Release);
        let overload = self.queue.len().cast_signed() - threads;
        if (overload > 0) && (threads < self.tmax.cast_signed()) {
            self.spawn_thread(threads.cast_unsigned());
        }
        Ok(())
    }
}

fn blocking_worker(queue: channel::Receiver<BlockingTask>) {
    Python::attach(|py| {
        while let Ok(task) = py.detach(|| queue.recv()) {
            task.run(py);
        }
    });
}

fn blocking_worker_idle(queue: channel::Receiver<BlockingTask>, timeout: time::Duration) {
    Python::attach(|py| {
        while let Ok(task) = py.detach(|| queue.recv_timeout(timeout)) {
            task.run(py);
        }
    });
}

#[cfg(not(Py_GIL_DISABLED))]
fn blocking_worker_with_metrics(queue: channel::Receiver<BlockingTask>, metrics: Arc<metrics::WorkerMetrics>) {
    Python::attach(|py| {
        let mut t_wait = time::Instant::now();
        while let Ok(task) = py.detach(|| {
            let t = time::Instant::now();
            let task = queue.recv();
            metrics
                .blocking_idle_cumul
                .fetch_add(t.elapsed().as_micros() as usize, atomic::Ordering::Release);
            if task.is_ok() {
                metrics.blocking_queue.fetch_sub(1, atomic::Ordering::Release);
            }
            t_wait = time::Instant::now();
            task
        }) {
            metrics
                .py_wait_cumul
                .fetch_add(t_wait.elapsed().as_micros() as usize, atomic::Ordering::Release);
            task.run(py);
            metrics
                .blocking_busy_cumul
                .fetch_add(t_wait.elapsed().as_micros() as usize, atomic::Ordering::Release);
        }
    });
}

#[cfg(Py_GIL_DISABLED)]
fn blocking_worker_with_metrics(queue: channel::Receiver<BlockingTask>, metrics: Arc<metrics::WorkerMetrics>) {
    Python::attach(|py| {
        while let Ok(task) = py.detach(|| {
            let t = time::Instant::now();
            let task = queue.recv();
            metrics
                .blocking_idle_cumul
                .fetch_add(t.elapsed().as_micros() as usize, atomic::Ordering::Release);
            if task.is_ok() {
                metrics.blocking_queue.fetch_sub(1, atomic::Ordering::Release);
            }
            task
        }) {
            let t = time::Instant::now();
            task.run(py);
            metrics
                .blocking_busy_cumul
                .fetch_add(t.elapsed().as_micros() as usize, atomic::Ordering::Release);
        }
    });
}

#[cfg(not(Py_GIL_DISABLED))]
fn blocking_worker_idle_with_metrics(
    queue: channel::Receiver<BlockingTask>,
    timeout: time::Duration,
    metrics: Arc<metrics::WorkerMetrics>,
) {
    Python::attach(|py| {
        let mut t_wait = time::Instant::now();
        while let Ok(task) = py.detach(|| {
            let t = time::Instant::now();
            let task = queue.recv_timeout(timeout);
            metrics
                .blocking_idle_cumul
                .fetch_add(t.elapsed().as_micros() as usize, atomic::Ordering::Release);
            if task.is_ok() {
                metrics.blocking_queue.fetch_sub(1, atomic::Ordering::Release);
            }
            t_wait = time::Instant::now();
            task
        }) {
            metrics
                .py_wait_cumul
                .fetch_add(t_wait.elapsed().as_micros() as usize, atomic::Ordering::Release);
            task.run(py);
            metrics
                .blocking_busy_cumul
                .fetch_add(t_wait.elapsed().as_micros() as usize, atomic::Ordering::Release);
        }
    });
}

#[cfg(Py_GIL_DISABLED)]
fn blocking_worker_idle_with_metrics(
    queue: channel::Receiver<BlockingTask>,
    timeout: time::Duration,
    metrics: Arc<metrics::WorkerMetrics>,
) {
    Python::attach(|py| {
        while let Ok(task) = py.detach(|| {
            let t = time::Instant::now();
            let task = queue.recv_timeout(timeout);
            metrics
                .blocking_idle_cumul
                .fetch_add(t.elapsed().as_micros() as usize, atomic::Ordering::Release);
            if task.is_ok() {
                metrics.blocking_queue.fetch_sub(1, atomic::Ordering::Release);
            }
            task
        }) {
            let t = time::Instant::now();
            task.run(py);
            metrics
                .blocking_busy_cumul
                .fetch_add(t.elapsed().as_micros() as usize, atomic::Ordering::Release);
        }
    });
}
