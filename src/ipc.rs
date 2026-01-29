#[cfg(not(Py_GIL_DISABLED))]
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
#[cfg(not(Py_GIL_DISABLED))]
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

#[cfg(not(Py_GIL_DISABLED))]
use anyhow::Result;
#[cfg(not(Py_GIL_DISABLED))]
use bytes::{Buf, BufMut, Bytes, BytesMut};
#[cfg(not(Py_GIL_DISABLED))]
use interprocess::unnamed_pipe;
#[cfg(not(Py_GIL_DISABLED))]
use serde::{Deserialize, Serialize};
#[cfg(not(Py_GIL_DISABLED))]
use std::{io::Cursor, sync::Arc};
#[cfg(not(Py_GIL_DISABLED))]
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};

use pyo3::prelude::*;

#[cfg(not(Py_GIL_DISABLED))]
#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct IPCReceiverHandle {
    pub id: usize,
    #[cfg(unix)]
    pub fd: OwnedFd,
    #[cfg(windows)]
    pub fd: OwnedHandle,
}

#[cfg(not(Py_GIL_DISABLED))]
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

#[cfg(not(Py_GIL_DISABLED))]
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
    pub fn new(id: usize, fd: Py<PyAny>) -> Self {
        let fd = unsafe { OwnedHandle::from_raw_handle(pyo3::ffi::PyLong_AsVoidPtr(fd.as_ptr())) };
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

#[cfg(not(Py_GIL_DISABLED))]
#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct IPCSenderHandle {
    #[cfg(unix)]
    pub fd: OwnedFd,
    #[cfg(windows)]
    pub fd: OwnedHandle,
}

#[cfg(not(Py_GIL_DISABLED))]
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

// FIXME: we should find a way to prevent Rust from closing the FD.
// impl Drop for IPCSenderHandle {
//     fn drop(&mut self) {
//         std::mem::forget(self.fd);
//     }
// }

#[cfg(not(Py_GIL_DISABLED))]
#[cfg(unix)]
#[pymethods]
impl IPCSenderHandle {
    #[new]
    pub fn new(fd: RawFd) -> Self {
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        Self { fd }
    }
}

#[cfg(not(Py_GIL_DISABLED))]
#[cfg(windows)]
#[pymethods]
impl IPCSenderHandle {
    #[new]
    pub fn new(fd: Py<PyAny>) -> Self {
        let fd = unsafe { OwnedHandle::from_raw_handle(pyo3::ffi::PyLong_AsVoidPtr(fd.as_ptr())) };
        Self { fd }
    }
}

#[cfg(Py_GIL_DISABLED)]
#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct IPCReceiverHandle {}

#[cfg(Py_GIL_DISABLED)]
#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct IPCSenderHandle {}

#[cfg(not(Py_GIL_DISABLED))]
#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum Message {
    // NOTE: potentially use IPC for more purposes
    Metrics(crate::metrics::MetricsData),
}

#[cfg(not(Py_GIL_DISABLED))]
fn pack_msg(msg: Message) -> Cursor<Bytes> {
    let mut buf = Vec::<u8>::new();
    ciborium::into_writer(&msg, &mut buf).unwrap();
    let mut rv = BytesMut::with_capacity(4);
    rv.put_u32(buf.len() as u32);
    rv.extend_from_slice(&buf);
    Cursor::new(rv.freeze())
}

#[cfg(not(Py_GIL_DISABLED))]
pub(crate) async fn write_msg<R: AsyncWrite + Unpin>(msg: Message, r: &mut R) -> Result<()> {
    let mut bytes = pack_msg(msg);
    r.write_all_buf(&mut bytes).await?;
    r.flush().await?;
    Ok(())
}

#[cfg(not(Py_GIL_DISABLED))]
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
