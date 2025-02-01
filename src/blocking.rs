use crossbeam_channel as channel;
use pyo3::prelude::*;
use std::thread;

pub(crate) struct BlockingTask {
    inner: Box<dyn FnOnce(Python) + Send + 'static>,
}

impl BlockingTask {
    pub fn new<T>(inner: T) -> BlockingTask
    where
        T: FnOnce(Python) + Send + 'static,
    {
        Self { inner: Box::new(inner) }
    }

    pub fn run(self, py: Python) {
        (self.inner)(py);
    }
}

pub(crate) struct BlockingRunner {
    queue: channel::Sender<BlockingTask>,
    #[cfg(Py_GIL_DISABLED)]
    sig: channel::Sender<()>,
}

impl BlockingRunner {
    #[cfg(not(Py_GIL_DISABLED))]
    pub fn new() -> Self {
        let queue = blocking_thread();
        Self { queue }
    }

    #[cfg(Py_GIL_DISABLED)]
    pub fn new() -> Self {
        let (sigtx, sigrx) = channel::bounded(1);
        let queue = blocking_thread(sigrx);
        Self { queue, sig: sigtx }
    }

    pub fn run<T>(&self, task: T) -> Result<(), channel::SendError<BlockingTask>>
    where
        T: FnOnce(Python) + Send + 'static,
    {
        self.queue.send(BlockingTask::new(task))
    }

    #[cfg(Py_GIL_DISABLED)]
    pub fn shutdown(&self) {
        _ = self.sig.send(());
    }
}

#[cfg(not(Py_GIL_DISABLED))]
fn blocking_loop(queue: channel::Receiver<BlockingTask>) {
    while let Ok(task) = queue.recv() {
        Python::with_gil(|py| task.run(py));
    }
}

// NOTE: for some reason, on no-gil callback watchers are not GCd until following req.
//       It's not clear atm wether this is an issue with pyo3, CPython itself, or smth
//       different in terms of pointers due to multi-threaded environment.
//       Thus, we need a signal to manually stop the loop and let the server shutdown.
//       The following function would be the intended one if we hadn't the issue just described.
//
// #[cfg(Py_GIL_DISABLED)]
// fn blocking_loop(queue: channel::Receiver<BlockingTask>) {
//     Python::with_gil(|py| {
//         while let Ok(task) = queue.recv() {
//             task.run(py);
//         }
//     });
// }
#[cfg(Py_GIL_DISABLED)]
fn blocking_loop(queue: channel::Receiver<BlockingTask>, sig: channel::Receiver<()>) {
    Python::with_gil(|py| loop {
        crossbeam_channel::select! {
            recv(queue) -> task => match task {
                Ok(task) => task.run(py),
                _ => break,
            },
            recv(sig) -> _ => break
        }
    });
}

#[cfg(not(Py_GIL_DISABLED))]
fn blocking_thread() -> channel::Sender<BlockingTask> {
    let (qtx, qrx) = channel::unbounded();
    thread::spawn(|| blocking_loop(qrx));

    qtx
}

#[cfg(Py_GIL_DISABLED)]
fn blocking_thread(sig: channel::Receiver<()>) -> channel::Sender<BlockingTask> {
    let (qtx, qrx) = channel::unbounded();
    thread::spawn(|| blocking_loop(qrx, sig));

    qtx
}
