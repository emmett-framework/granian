#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use pyo3::prelude::*;

mod asgi;
mod callbacks;
mod conversion;
mod http;
mod rsgi;
mod runtime;
mod tcp;
mod tls;
mod utils;
mod workers;
mod ws;
mod wsgi;

#[pymodule]
fn _granian(py: Python, module: &PyModule) -> PyResult<()> {
    asgi::init_pymodule(module)?;
    rsgi::init_pymodule(py, module)?;
    tcp::init_pymodule(module)?;
    workers::init_pymodule(module)?;
    wsgi::init_pymodule(py, module)?;
    Ok(())
}
