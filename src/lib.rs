#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use pyo3::prelude::*;

mod asgi;
mod callbacks;
mod http;
mod rsgi;
mod runtime;
mod tls;
mod tcp;
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

    #[cfg(not(PyPy))]
    pyo3::prepare_freethreaded_python();

    Ok(())
}
