use crossbeam_channel as channel;
use std::thread;

pub(crate) struct BlockingTask {
    inner: Box<dyn FnOnce() + Send + 'static>,
}

impl BlockingTask {
    pub fn new<T>(inner: T) -> BlockingTask
    where
        T: FnOnce() + Send + 'static,
    {
        Self { inner: Box::new(inner) }
    }

    pub fn run(self) {
        (self.inner)();
    }
}

#[derive(Clone)]
pub(crate) struct BlockingRunner {
    queue: channel::Sender<BlockingTask>,
}

impl BlockingRunner {
    pub fn new() -> Self {
        let queue = blocking_thread();
        Self { queue }
    }

    pub fn run<T>(&self, task: T) -> Result<(), channel::SendError<BlockingTask>>
    where
        T: FnOnce() + Send + 'static,
    {
        self.queue.send(BlockingTask::new(task))
    }
}

fn bloking_loop(queue: channel::Receiver<BlockingTask>) {
    while let Ok(task) = queue.recv() {
        task.run();
    }
}

fn blocking_thread() -> channel::Sender<BlockingTask> {
    let (qtx, qrx) = channel::unbounded();
    thread::spawn(|| bloking_loop(qrx));
    qtx
}
