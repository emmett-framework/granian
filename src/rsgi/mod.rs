use pyo3::prelude::*;

mod callbacks;
pub(crate) mod conversion;
mod errors;
mod http;
pub(crate) mod io;
pub(crate) mod serve;
mod types;

pub(crate) fn init_pymodule(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    module.add("RSGIProtocolError", py.get_type::<errors::RSGIProtocolError>())?;
    module.add("RSGIProtocolClosed", py.get_type::<errors::RSGIProtocolClosed>())?;
    module.add_class::<io::RSGIHTTPProtocol>()?;
    module.add_class::<io::RSGIHTTPStreamTransport>()?;
    module.add_class::<io::RSGIWebsocketProtocol>()?;
    module.add_class::<io::RSGIWebsocketTransport>()?;
    module.add_class::<types::RSGIHeaders>()?;
    module.add_class::<types::RSGIHTTPScope>()?;
    module.add_class::<types::RSGIWebsocketScope>()?;

    Ok(())
}
