use pyo3::prelude::*;

pub(crate) mod conversion;
mod errors;
mod http;
mod interop;
mod io;
pub(crate) mod serve;
pub(crate) mod types;
mod utils;

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<io::ASGIHTTPProtocol>()?;
    module.add_class::<io::ASGIWebsocketProtocol>()?;

    Ok(())
}
