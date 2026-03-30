use pyo3::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
#[pyclass(frozen, subclass, module = "granian._granian")]
pub(super) struct RSGIApp {
    pub on_request: Arc<Py<PyAny>>,
    // pub on_data: Arc<Py<PyAny>>,
    // pub on_disconnect: Arc<Py<PyAny>>,
}

impl RSGIApp {
    pub(super) fn to_callback_impl(&self, rt: crate::runtime::RuntimeRef) -> super::callbacks::CallbackImpl {
        super::callbacks::CallbackImpl::from_app(rt, self)
    }
}

#[pymethods]
impl RSGIApp {
    #[new]
    fn new(app: Bound<PyAny>) -> PyResult<Self> {
        let py = app.py();
        Ok(Self {
            on_request: app.getattr(pyo3::intern!(py, "on_request"))?.unbind().into(),
            // on_data: app.getattr(pyo3::intern!(py, "on_data"))?.unbind().into(),
            // on_disconnect: app.getattr(pyo3::intern!(py, "on_disconnect"))?.unbind().into(),
        })
    }
}

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<RSGIApp>()?;
    module.add_class::<super::callbacks::PyAbortHandle>()?;

    Ok(())
}
