use pyo3::prelude::*;

mod callbacks;
mod errors;
mod http;
pub(crate) mod serve;
mod types;

pub(crate) fn init_pymodule(py: Python, module: &PyModule) -> PyResult<()> {
    module.add("WSGIProtocolError", py.get_type::<errors::WSGIProtocolError>())?;
    module.add_class::<types::WSGIScope>()?;

    Ok(())
}
