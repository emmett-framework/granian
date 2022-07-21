use pyo3::prelude::*;

mod callbacks;
mod errors;
mod http;
mod io;
pub(crate) mod serve;
mod types;

pub(crate) fn init_pymodule(module: &PyModule) -> PyResult<()> {
    module.add_class::<io::ASGIHTTPProtocol>()?;
    module.add_class::<io::ASGIWebsocketProtocol>()?;
    module.add_class::<types::ASGIScope>()?;

    Ok(())
}
