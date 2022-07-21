use pyo3::prelude::*;

mod callbacks;
mod errors;
mod http;
mod io;
pub(crate) mod serve;
mod types;

pub(crate) fn init_pymodule(module: &PyModule) -> PyResult<()> {
    module.add_class::<io::RSGIHTTPProtocol>()?;
    module.add_class::<io::RSGIWebsocketProtocol>()?;
    module.add_class::<io::RSGIWebsocketTransport>()?;
    module.add_class::<types::RSGIHeaders>()?;
    module.add_class::<types::RSGIScope>()?;

    Ok(())
}
