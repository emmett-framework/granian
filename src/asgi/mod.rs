use pyo3::prelude::*;

mod callbacks;
mod errors;
mod http;
mod io;
pub(crate) mod serve;
mod types;

pub(crate) fn build_pymodule(py: Python) -> PyResult<&PyModule> {
    let module = PyModule::new(py, "asgi")?;

    module.add_class::<io::Sender>()?;
    module.add_class::<types::Scope>()?;

    Ok(module)
}
