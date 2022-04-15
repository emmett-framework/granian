use bytes::Buf;
use hyper::{Body, Request};
use pyo3::prelude::*;
use pyo3::types::{PyBytes};
use std::sync::{Arc};
use tokio::sync::{Mutex};

#[pyclass(module="granian.rsgi")]
pub(crate) struct Receiver {
    request: Arc<Mutex<Request<Body>>>
}

impl Receiver {
    pub fn new(request: Request<Body>) -> Self {
        Self {
            request: Arc::new(Mutex::new(request))
        }
    }
}

#[pymethods]
impl Receiver {
    fn __call__<'p>(&self, py: Python<'p>) -> PyResult<&'p PyAny> {
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
}
