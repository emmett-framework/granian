use pyo3::prelude::*;

mod app;
mod callbacks;
mod http;
mod io;
mod serve;
mod workers;

pub(crate) fn init_pymodule(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<io::RSGI2HTTPProtocol>()?;
    module.add_class::<io::RSGIHTTPReader>()?;
    module.add_class::<io::RSGIHTTPWriter>()?;
    // module.add_class::<io::RSGIHTTPStreamTransport>()?;
    // module.add_class::<io::RSGIWebsocketProtocol>()?;
    // module.add_class::<io::RSGIWebsocketTransport>()?;
    app::init_pymodule(module)?;
    workers::init_pymodule(module)?;

    Ok(())
}
