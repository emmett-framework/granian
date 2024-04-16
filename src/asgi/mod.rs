use pyo3::prelude::*;

mod callbacks;
mod errors;
mod http;
mod io;
pub(crate) mod serve;
mod types;
mod utils;

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<io::ASGIHTTPProtocol>()?;
    module.add_class::<io::ASGIWebsocketProtocol>()?;

    Ok(())
}
