#[cfg(not(all(target_os="linux", target_arch="aarch64")))]
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

#[pymodule]
fn _granian(_py: Python, module: &PyModule) -> PyResult<()> {
    asgi::init_pymodule(module)?;
    rsgi::init_pymodule(module)?;
    tcp::init_pymodule(module)?;
    workers::init_pymodule(module)?;

    pyo3::prepare_freethreaded_python();

    Ok(())
}
