use bytes::Buf;
use hyper::{Body, Request};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::runtime::{RuntimeRef, future_into_py};

#[pyclass(module="granian.io")]
pub(crate) struct Receiver {
    // thread_mode: ThreadIsolation,
    rt: RuntimeRef,
    request: Arc<Mutex<Request<Body>>>
}

impl Receiver {
    pub fn new(rt: RuntimeRef, request: Request<Body>) -> Self {
        Self {
            rt: rt,
            request: Arc::new(Mutex::new(request))
        }
    }
}

#[pymethods]
impl Receiver {
    fn __call__<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let req_ref = self.request.clone();
        future_into_py(self.rt.clone(), py, async move {
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

pub(crate) fn build_pymodule(py: Python) -> PyResult<&PyModule> {
    let module = PyModule::new(py, "io")?;

    module.add_class::<Receiver>()?;

    Ok(module)
}
