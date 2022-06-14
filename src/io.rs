use bytes::Buf;
use hyper::{Body, Request};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::runtime::{ThreadIsolation, future_into_py};

#[pyclass(module="granian.io")]
pub(crate) struct Receiver {
    thread_mode: ThreadIsolation,
    request: Arc<Mutex<Request<Body>>>
}

impl Receiver {
    pub fn new(thread_mode: ThreadIsolation, request: Request<Body>) -> Self {
        Self {
            thread_mode: thread_mode,
            request: Arc::new(Mutex::new(request))
        }
    }

    fn receive_mt_runtime<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let req_ref = self.request.clone();
        pyo3_asyncio::tokio::future_into_py(py, async move {
            let mut req = req_ref.lock().await;
            let mut body = hyper::body::to_bytes(&mut *req).await.unwrap();
            Ok(Python::with_gil(|py| {
                // PyBytes::new(py, &body.to_vec());
                PyBytes::new_with(py, body.len(), |bytes: &mut [u8]| {
                    body.copy_to_slice(bytes);
                    Ok(())
                }).unwrap().as_ref().to_object(py)
            }))
        })
    }

    fn receive_st_runtime<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let req_ref = self.request.clone();
        future_into_py(py, async move {
            let mut req = req_ref.lock().await;
            let mut body = hyper::body::to_bytes(&mut *req).await.unwrap();
            Ok(Python::with_gil(|py| {
                // PyBytes::new(py, &body.to_vec());
                PyBytes::new_with(py, body.len(), |bytes: &mut [u8]| {
                    body.copy_to_slice(bytes);
                    Ok(())
                }).unwrap().as_ref().to_object(py)
            }))
        })
    }
}

#[pymethods]
impl Receiver {
    fn __call__<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        match self.thread_mode {
            ThreadIsolation::Runtime => self.receive_mt_runtime(py),
            ThreadIsolation::Worker => self.receive_st_runtime(py)
        }
    }
}

pub(crate) fn build_pymodule(py: Python) -> PyResult<&PyModule> {
    let module = PyModule::new(py, "io")?;

    module.add_class::<Receiver>()?;

    Ok(module)
}
