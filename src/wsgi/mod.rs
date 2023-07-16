use pyo3::prelude::*;

mod callbacks;
mod http;
pub(crate) mod serve;
mod types;

pub(crate) fn init_pymodule(_py: Python, module: &PyModule) -> PyResult<()> {
    module.add_class::<types::WSGIScope>()?;

    Ok(())
}
