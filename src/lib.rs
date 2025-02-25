#[cfg(not(any(
    target_env = "musl",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "windows",
    feature = "mimalloc"
)))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(feature = "mimalloc")]
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

#[cfg(not(Py_GIL_DISABLED))]
const BUILD_GIL: bool = true;
#[cfg(Py_GIL_DISABLED)]
const BUILD_GIL: bool = false;

pub fn get_granian_version() -> &'static str {
    static GRANIAN_VERSION: OnceLock<String> = OnceLock::new();

    GRANIAN_VERSION.get_or_init(|| {
        let version = env!("CARGO_PKG_VERSION");
        version.replace("-alpha", "a").replace("-beta", "b")
    })
}

#[pymodule(gil_used = false)]
fn _granian(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    module.add("__version__", get_granian_version())?;
    module.add("BUILD_GIL", BUILD_GIL)?;
    module.add_class::<callbacks::CallbackScheduler>()?;
    asgi::init_pymodule(module)?;
    rsgi::init_pymodule(py, module)?;
    tcp::init_pymodule(module)?;
    workers::init_pymodule(module)?;
    Ok(())
}
