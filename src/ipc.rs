#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};

use anyhow::Result;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use interprocess::unnamed_pipe;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::{io::Cursor, sync::Arc};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct IPCReceiverHandle {
    pub id: usize,
    #[cfg(unix)]
    pub fd: OwnedFd,
    #[cfg(windows)]
    pub fd: OwnedHandle,
}

impl IPCReceiverHandle {
    fn to_receiver(&self) -> Arc<Mutex<unnamed_pipe::tokio::Recver>> {
        #[cfg(unix)]
        let fd: OwnedFd = unsafe { OwnedFd::from_raw_fd(self.fd.as_raw_fd()) };
        #[cfg(windows)]
        let fd: OwnedHandle = unsafe { OwnedHandle::from_raw_handle(self.fd.as_raw_handle()) };
        let receiver = unnamed_pipe::tokio::Recver::try_from(fd).unwrap();
        Arc::new(Mutex::new(receiver))
    }
}

#[pymethods]
impl IPCReceiverHandle {
    #[cfg(unix)]
    #[new]
    pub fn new(id: usize, fd: RawFd) -> Self {
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        Self { id, fd }
    }

    #[cfg(windows)]
    #[new]
    pub fn new(id: usize, fd: i32) -> Self {
        let fd = unsafe { OwnedHandle::from_raw_handle(fd) };
        Self { id, fd }
    }

    fn run(
        pyself: Py<Self>,
        py: Python,
        sig: Py<crate::workers::WorkerSignal>,
        metrics_aggregator: Py<crate::metrics::MetricsAggregator>,
    ) {
        let pynone = py.None().into_any();
        let handle = pyself.clone_ref(py);
        let metrics_aggregator = Arc::new(metrics_aggregator);

        std::thread::spawn(move || {
            let rt = crate::runtime::init_runtime_st(1, 0, 0, Arc::new(pynone), None);
            let local = tokio::task::LocalSet::new();

            let idx = handle.get().id;
            let mut pyrx = {
                let guard = sig.get().rx.lock().unwrap();
                guard.clone().unwrap()
            };

            crate::runtime::block_on_local(&rt, local, async move {
                let ipc = handle.get().to_receiver();
                loop {
                    let ipc = ipc.clone();
                    let metrics_aggregator = metrics_aggregator.clone();

                    tokio::select! {
                        biased;
                        () = async move {
                            let mut receiver = ipc.lock().await;
                            // TODO: log unexpected messages?
                            // match read_msg(&mut *receiver).await {
                            //     Ok(Message::Metrics(data)) => metrics_aggregator.get().collect(idx, data),
                            //     _ => {}
                            // }
                            if let Ok(Message::Metrics(data)) = read_msg(&mut *receiver).await {
                                metrics_aggregator.get().collect(idx, data);
                            }
                        } => {},
                        _ = pyrx.changed() => break,
                    }
                }

                Python::attach(|_| {
                    drop(ipc);
                    drop(metrics_aggregator);
                });
            });

            Python::attach(|_| {
                drop(sig);
                drop(rt);
            });
        });
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct IPCSenderHandle {
    #[cfg(unix)]
    pub fd: OwnedFd,
    #[cfg(windows)]
    pub fd: OwnedHandle,
}

impl IPCSenderHandle {
    pub(crate) fn to_sender(&self) -> Arc<Mutex<unnamed_pipe::tokio::Sender>> {
        #[cfg(unix)]
        let fd: OwnedFd = unsafe { OwnedFd::from_raw_fd(self.fd.as_raw_fd()) };
        #[cfg(windows)]
        let fd: OwnedHandle = unsafe { OwnedHandle::from_raw_handle(self.fd.as_raw_handle()) };
        let sender = unnamed_pipe::tokio::Sender::try_from(fd).unwrap();
        Arc::new(Mutex::new(sender))
    }
}

// impl Drop for IPCSenderHandle {
//     fn drop(&mut self) {
//         std::mem::forget(self.fd);
//     }
// }

#[cfg(unix)]
#[pymethods]
impl IPCSenderHandle {
    #[new]
    pub fn new(fd: RawFd) -> Self {
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        Self { fd }
    }
}

#[cfg(windows)]
#[pymethods]
impl IPCSenderHandle {
    #[new]
    pub fn new(fd: i32) -> Self {
        let fd = unsafe { OwnedHandle::from_raw_handle(fd) };
        Self { fd }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum Message {
    // NOTE: potentially use IPC for more purposes
    Metrics(crate::metrics::MetricsData),
}

fn pack_msg(msg: Message) -> Cursor<Bytes> {
    let mut buf = Vec::<u8>::new();
    ciborium::into_writer(&msg, &mut buf).unwrap();
    let mut rv = BytesMut::with_capacity(4);
    rv.put_u32(buf.len() as u32);
    rv.extend_from_slice(&buf);
    Cursor::new(rv.freeze())
}

pub(crate) async fn write_msg<R: AsyncWrite + Unpin>(msg: Message, r: &mut R) -> Result<()> {
    let mut bytes = pack_msg(msg);
    r.write_all_buf(&mut bytes).await?;
    r.flush().await?;
    Ok(())
}

pub(crate) async fn read_msg<R: AsyncRead + Unpin>(r: &mut R) -> Result<Message> {
    let mut bytes = [0u8; 4];
    r.read_exact(&mut bytes).await?;
    let len = (&bytes[..]).get_u32();
    let mut buf = vec![0; len as usize];
    r.read_exact(&mut buf).await?;
    Ok(ciborium::from_reader(&buf[..])?)
}

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<IPCReceiverHandle>()?;
    module.add_class::<IPCSenderHandle>()?;

    Ok(())
}
