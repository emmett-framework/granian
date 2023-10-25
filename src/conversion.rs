use pyo3::prelude::*;
use std::ops::{Deref, DerefMut};

pub(crate) struct BytesToPy(pub bytes::Bytes);

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
