#[cfg(not(all(target_os="linux", target_arch="aarch64")))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use pyo3::prelude::*;

mod asgi;
mod callbacks;
mod http;
mod rsgi;
mod runtime;
mod tcp;
mod utils;
mod workers;
mod ws;

#[pymodule]
fn granian(py: Python, module: &PyModule) -> PyResult<()> {
    module.add_submodule(asgi::build_pymodule(py)?)?;
    module.add_submodule(rsgi::build_pymodule(py)?)?;
    module.add_submodule(tcp::build_pymodule(py)?)?;
    module.add_submodule(workers::build_pymodule(py)?)?;

    pyo3::prepare_freethreaded_python();

    Ok(())
}
