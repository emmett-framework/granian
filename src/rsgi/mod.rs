use pyo3::prelude::*;

mod callbacks;
mod http;
mod io;
pub(crate) mod serve;
mod types;

pub(crate) fn build_pymodule(py: Python) -> PyResult<&PyModule> {
    let module = PyModule::new(py, "rsgi")?;

    module.add_class::<io::Receiver>()?;
    module.add_class::<types::Scope>()?;

    Ok(module)
}
