#[cfg(all(feature = "jemalloc", not(feature = "mimalloc")))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(all(feature = "mimalloc", not(feature = "jemalloc")))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use pyo3::prelude::*;
use std::sync::OnceLock;

pub use http::{HTTPRequest, HTTPResponse, HTTPResponseBody};
pub use native::{NativeContext, NativeHandler, NativeRouteKind, register_native_route};
pub use net::SockAddr;

mod asgi;
mod asyncio;
mod blocking;
mod callbacks;
mod conversion;
mod files;
pub mod http;
mod ipc;
mod metrics;
mod native;
mod net;
mod rsgi;
mod runtime;
mod serve;
mod sys;
mod tls;
mod utils;
mod workers;
pub mod ws;
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

#[cfg(feature = "python-module")]
#[pymodule(gil_used = false)]
fn _granian(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    init_py_module(py, module)?;

    // The reference routes are only registered by the Granian module itself:
    // crates embedding Granian as a library are expected to register their
    // own ones.
    native::register_reference_routes();

    Ok(())
}

/// Initialise the Granian Python module.
///
/// This is the entry point to be used by crates embedding Granian as a
/// library, in order to provide the `granian._granian` module with their own
/// extensions (i.e. the native routes).
///
/// # Errors
///
/// Returns an error if the module cannot be initialised.
pub fn init_py_module(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    module.add("__version__", get_granian_version())?;
    module.add("BUILD_GIL", BUILD_GIL)?;
    module.add_class::<callbacks::CallbackScheduler>()?;
    asgi::init_pymodule(module)?;
    ipc::init_pymodule(module)?;
    metrics::init_pymodule(module)?;
    rsgi::init_pymodule(py, module)?;
    sys::init_pymodule(module)?;
    net::init_pymodule(module)?;
    workers::init_pymodule(module)?;
    Ok(())
}
