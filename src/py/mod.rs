use pyo3::prelude::*;

pub(crate) mod asyncio;
pub(crate) mod interop;

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<interop::PyAbortHandle>()?;
    module.add_class::<interop::PyApp>()?;

    Ok(())
}
