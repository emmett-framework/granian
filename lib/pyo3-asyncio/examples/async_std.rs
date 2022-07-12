use pyo3::prelude::*;

#[pyo3_asyncio::async_std::main]
async fn main() -> PyResult<()> {
    let fut = Python::with_gil(|py| {
        let asyncio = py.import("asyncio")?;

        // convert asyncio.sleep into a Rust Future
        pyo3_asyncio::async_std::into_future(asyncio.call_method1("sleep", (1.into_py(py),))?)
    })?;

    println!("sleeping for 1s");
    fut.await?;
    println!("done");

    Ok(())
}
