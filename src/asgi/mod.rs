use pyo3::prelude::*;

mod callbacks;
pub(crate) mod conversion;
mod errors;
mod http;
mod io;
pub(crate) mod serve;
pub(crate) mod types;
mod utils;

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<io::ASGIHTTPProtocol>()?;
    module.add_class::<io::ASGIWebsocketProtocol>()?;

    Ok(())
}
