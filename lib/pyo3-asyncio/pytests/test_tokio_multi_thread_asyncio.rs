mod common;
mod tokio_asyncio;

use pyo3::prelude::*;

fn main() -> pyo3::PyResult<()> {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| pyo3_asyncio::tokio::run(py, pyo3_asyncio::testing::main()))
}
