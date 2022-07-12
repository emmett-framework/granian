// FIXME - is there a way to document custom PyO3 exceptions?
#[allow(missing_docs)]
mod exceptions {
    use pyo3::{create_exception, exceptions::PyException};

    create_exception!(pyo3_asyncio, RustPanic, PyException);
}

pub use exceptions::RustPanic;
