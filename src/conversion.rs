use pyo3::prelude::*;
use std::ops::{Deref, DerefMut};

pub(crate) struct BytesToPy(pub bytes::Bytes);
pub(crate) struct HTTPVersionToPy(pub hyper::Version);

impl Deref for BytesToPy {
    type Target = bytes::Bytes;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BytesToPy {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoPy<PyObject> for BytesToPy {
    #[inline]
    fn into_py(self, py: Python) -> PyObject {
        (&self[..]).into_py(py)
    }
}

impl ToPyObject for HTTPVersionToPy {
    #[inline]
    fn to_object(&self, py: Python<'_>) -> PyObject {
        match self.0 {
            hyper::Version::HTTP_10 => "HTTP/1",
            hyper::Version::HTTP_11 => "HTTP/1.1",
            hyper::Version::HTTP_2 => "HTTP/2",
            hyper::Version::HTTP_3 => "HTTP/3",
            _ => "HTTP/1",
        }
        .into_py(py)
    }
}
