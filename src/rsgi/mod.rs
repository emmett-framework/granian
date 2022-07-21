use pyo3::prelude::*;

mod callbacks;
mod errors;
mod http;
mod io;
pub(crate) mod serve;
mod types;

pub(crate) fn build_pymodule(py: Python) -> PyResult<&PyModule> {
    let module = PyModule::new(py, "rsgi")?;

    module.add_class::<io::HTTPProtocol>()?;
    module.add_class::<io::WebsocketProtocol>()?;
    module.add_class::<types::Headers>()?;
    module.add_class::<types::Scope>()?;

    Ok(module)
}
