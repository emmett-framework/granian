//! Example of a crate embedding Granian as a library in order to serve
//! *native routes*: paths handled directly by the Rust side, without involving
//! the Python application.
//!
//! The crate builds a Python extension module named `granian._granian`, which
//! replaces the one shipped with the Granian package: the Python side is
//! entirely stock Granian, while the native routes are provided here.

use pyo3::prelude::*;

mod consumers;
mod views;

#[pyo3::pymodule(gil_used = false)]
fn _granian(py: Python, module: &Bound<PyModule>) -> PyResult<()> {
    // Initialise the Granian module: this provides the workers, the callback
    // scheduler and everything else the Python side expects to find.
    granian::init_py_module(py, module)?;

    // Register the native routes: they are served by this crate, without ever
    // reaching the Python application.
    granian::register_native_route(
        "/native_view",
        granian::NativeRouteKind::Http,
        views::info_view,
    );
    granian::register_native_route(
        "/native_ws",
        granian::NativeRouteKind::Websocket,
        consumers::broadcast_consumer(),
    );

    Ok(())
}
