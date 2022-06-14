use pyo3::prelude::*;

mod callbacks;
mod http;
pub(crate) mod serve;
mod types;

pub(crate) fn build_pymodule(py: Python) -> PyResult<&PyModule> {
    let module = PyModule::new(py, "rsgi")?;

    module.add_class::<types::Headers>()?;
    module.add_class::<types::Scope>()?;

    Ok(module)
}
