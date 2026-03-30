use pyo3::prelude::*;

pub(crate) mod conversion;
pub(crate) mod errors;
mod http;
mod interop;
pub(crate) mod io;
pub(crate) mod serve;
pub(crate) mod types;

pub(crate) fn init_pymodule(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    module.add("RSGIProtocolError", py.get_type::<errors::RSGIProtocolError>())?;
    module.add("RSGIProtocolClosed", py.get_type::<errors::RSGIProtocolClosed>())?;
    module.add_class::<io::HTTPProtocol>()?;
    module.add_class::<io::HTTPReader>()?;
    module.add_class::<io::HTTPWriter>()?;
    module.add_class::<io::WebsocketProtocol>()?;
    module.add_class::<io::WebsocketReader>()?;
    module.add_class::<io::WebsocketWriter>()?;
    module.add_class::<types::RSGIHeaders>()?;
    module.add_class::<types::RSGIHTTPScope>()?;
    module.add_class::<types::RSGIWebsocketScope>()?;

    Ok(())
}
