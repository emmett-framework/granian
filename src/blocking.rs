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
    birth: time::Instant,
    queue: channel::Sender<BlockingTask>,
    tq: channel::Receiver<BlockingTask>,
    threads: Arc<atomic::AtomicUsize>,
    tmax: usize,
    idle_timeout: time::Duration,
    spawning: atomic::AtomicBool,
    spawn_tick: atomic::AtomicU64,
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
            birth: time::Instant::now(),
            spawning: false.into(),
            spawn_tick: 0.into(),
            idle_timeout: time::Duration::from_secs(idle_timeout),
            metrics: (),
        };

        // always spawn the first thread
        thread::spawn(move || blocking_worker(qrx));

        ret
    }

    #[inline(always)]
    fn spawn_thread(&self) {
        let tick = self.birth.elapsed().as_micros() as u64;
        if tick - self.spawn_tick.load(atomic::Ordering::Relaxed) < 350 {
            return;
        }
        if self
            .spawning
            .compare_exchange(false, true, atomic::Ordering::Relaxed, atomic::Ordering::Relaxed)
            .is_err()
        {
            return;
        }

        let queue = self.tq.clone();
        let tcount = self.threads.clone();
        let timeout = self.idle_timeout;
        thread::spawn(move || {
            tcount.fetch_add(1, atomic::Ordering::Release);
            blocking_worker_idle(queue, timeout);
            tcount.fetch_sub(1, atomic::Ordering::Release);
        });

        self.spawn_tick
            .store(self.birth.elapsed().as_micros() as u64, atomic::Ordering::Relaxed);
        self.spawning.store(false, atomic::Ordering::Relaxed);
    }

    #[inline]
    pub fn run<T>(&self, task: T) -> Result<(), channel::SendError<BlockingTask>>
    where
        T: FnOnce(Python) + Send + 'static,
    {
        self.queue.send(BlockingTask::new(task))?;
        if self.queue.len() > 1 && self.threads.load(atomic::Ordering::Acquire) < self.tmax {
            self.spawn_thread();
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
            birth: time::Instant::now(),
            spawning: false.into(),
            spawn_tick: 0.into(),
            idle_timeout: time::Duration::from_secs(idle_timeout),
            metrics: metrics.clone(),
        };

        // always spawn the first thread
        thread::spawn(move || blocking_worker_with_metrics(qrx, metrics));
        ret.metrics.blocking_threads.store(1, atomic::Ordering::Release);

        ret
    }

    #[inline(always)]
    fn spawn_thread(&self) {
        let tick = self.birth.elapsed().as_micros() as u64;
        if tick - self.spawn_tick.load(atomic::Ordering::Relaxed) < 350 {
            return;
        }
        if self
            .spawning
            .compare_exchange(false, true, atomic::Ordering::Relaxed, atomic::Ordering::Relaxed)
            .is_err()
        {
            return;
        }

        let queue = self.tq.clone();
        let metrics = self.metrics.clone();

        let timeout = self.idle_timeout;
        thread::spawn(move || {
            metrics.blocking_threads.fetch_add(1, atomic::Ordering::Release);
            blocking_worker_idle_with_metrics(queue, timeout, metrics.clone());
            metrics.blocking_threads.fetch_sub(1, atomic::Ordering::Release);
        });

        self.spawn_tick
            .store(self.birth.elapsed().as_micros() as u64, atomic::Ordering::Relaxed);
        self.spawning.store(false, atomic::Ordering::Relaxed);
    }

    #[inline]
    pub fn run<T>(&self, task: T) -> Result<(), channel::SendError<BlockingTask>>
    where
        T: FnOnce(Python) + Send + 'static,
    {
        self.queue.send(BlockingTask::new(task))?;
        let qlen = self.metrics.blocking_queue.fetch_add(1, atomic::Ordering::Release);
        if qlen > 0 && self.metrics.blocking_threads.load(atomic::Ordering::Acquire) < self.tmax {
            self.spawn_thread();
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
