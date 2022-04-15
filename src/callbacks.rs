use pyo3::prelude::*;

#[derive(Clone)]
pub(crate) struct CallbackWrapper {
    pub callback: PyObject,
    pub context: pyo3_asyncio::TaskLocals
}

impl CallbackWrapper {
    pub(crate) fn new(callback: PyObject, event_loop: &PyAny, context: &PyAny) -> Self {
        Self {
            callback: callback,
            context: pyo3_asyncio::TaskLocals::new(event_loop).with_context(context)
        }
    }
}
