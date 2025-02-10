use crossbeam_channel as channel;
use pyo3::prelude::*;
use std::{
    sync::{atomic, Arc},
    thread, time,
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
    Mono(BlockingRunnerMono),
    Pool(BlockingRunnerPool),
}

impl BlockingRunner {
    pub fn new(max_threads: usize, idle_timeout: u64) -> Self {
        match max_threads {
            0 => Self::Empty,
            1 => Self::Mono(BlockingRunnerMono::new()),
            _ => Self::Pool(BlockingRunnerPool::new(max_threads, idle_timeout)),
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
            Self::Empty => Ok(()),
        }
    }
}

pub(crate) struct BlockingRunnerMono {
    queue: channel::Sender<BlockingTask>,
}

impl BlockingRunnerMono {
    pub fn new() -> Self {
        let (qtx, qrx) = channel::unbounded();
        let ret = Self { queue: qtx };

        #[cfg(not(Py_GIL_DISABLED))]
        thread::spawn(move || blocking_worker(qrx));
        #[cfg(Py_GIL_DISABLED)]
        thread::spawn(move || Python::with_gil(|py| blocking_worker(qrx, py)));

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

pub(crate) struct BlockingRunnerPool {
    birth: time::Instant,
    queue: channel::Sender<BlockingTask>,
    tq: channel::Receiver<BlockingTask>,
    threads: Arc<atomic::AtomicUsize>,
    tmax: usize,
    idle_timeout: time::Duration,
    spawning: atomic::AtomicBool,
    spawn_tick: atomic::AtomicU64,
}

impl BlockingRunnerPool {
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
        };

        // always spawn the first thread
        #[cfg(not(Py_GIL_DISABLED))]
        thread::spawn(move || blocking_worker(qrx));
        #[cfg(Py_GIL_DISABLED)]
        thread::spawn(move || Python::with_gil(|py| blocking_worker(qrx, py)));

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
        #[cfg(not(Py_GIL_DISABLED))]
        thread::spawn(move || {
            tcount.fetch_add(1, atomic::Ordering::Release);
            blocking_worker_idle(queue, timeout);
            tcount.fetch_sub(1, atomic::Ordering::Release);
        });
        #[cfg(Py_GIL_DISABLED)]
        thread::spawn(move || {
            Python::with_gil(|py| {
                tcount.fetch_add(1, atomic::Ordering::Release);
                blocking_worker_idle(queue, timeout, py);
                tcount.fetch_sub(1, atomic::Ordering::Release);
            });
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

#[cfg(not(Py_GIL_DISABLED))]
fn blocking_worker(queue: channel::Receiver<BlockingTask>) {
    while let Ok(task) = queue.recv() {
        Python::with_gil(|py| task.run(py));
    }
}

#[cfg(Py_GIL_DISABLED)]
fn blocking_worker(queue: channel::Receiver<BlockingTask>, py: Python) {
    while let Ok(task) = queue.recv() {
        task.run(py);
    }
}

#[cfg(not(Py_GIL_DISABLED))]
fn blocking_worker_idle(queue: channel::Receiver<BlockingTask>, timeout: time::Duration) {
    while let Ok(task) = queue.recv_timeout(timeout) {
        Python::with_gil(|py| task.run(py));
    }
}

// NOTE: for some reason, on no-gil callback watchers are not GCd until following req.
//       This seems to cause the underlying function to hang on exiting the
//       `Python::with_gil` block under some circumstances (seems correlated to no of threads).
//       It's not clear atm wether this is an issue with pyo3, CPython itself, or smth
//       different in terms of pointers due to multi-threaded environment.
//       Given the whole free-threaded Python support is experimental, we avoid strange
//       hacks and wait for additional info.
#[cfg(Py_GIL_DISABLED)]
fn blocking_worker_idle(queue: channel::Receiver<BlockingTask>, timeout: time::Duration, py: Python) {
    while let Ok(task) = queue.recv_timeout(timeout) {
        task.run(py);
    }
}
