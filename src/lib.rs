#[cfg(not(any(target_os = "freebsd", target_os = "windows")))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(any(target_os = "freebsd", target_os = "windows"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use pyo3::prelude::*;
use std::sync::OnceLock;

mod asgi;
mod asyncio;
mod blocking;
mod callbacks;
mod conversion;
mod http;
mod io;
mod rsgi;
mod runtime;
mod tcp;
mod tls;
mod utils;
mod workers;
mod ws;
mod wsgi;

pub fn get_granian_version() -> &'static str {
    static GRANIAN_VERSION: OnceLock<String> = OnceLock::new();

    GRANIAN_VERSION.get_or_init(|| {
        let version = env!("CARGO_PKG_VERSION");
        version.replace("-alpha", "a").replace("-beta", "b")
    })
}

#[pymodule]
fn _granian(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    module.add("__version__", get_granian_version())?;
    asgi::init_pymodule(module)?;
    rsgi::init_pymodule(py, module)?;
    tcp::init_pymodule(module)?;
    workers::init_pymodule(module)?;
    Ok(())
}
