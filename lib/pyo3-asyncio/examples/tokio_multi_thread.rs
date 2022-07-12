use pyo3::prelude::*;

#[pyo3_asyncio::tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> PyResult<()> {
    let fut = Python::with_gil(|py| {
        let asyncio = py.import("asyncio")?;

        // convert asyncio.sleep into a Rust Future
        pyo3_asyncio::tokio::into_future(asyncio.call_method1("sleep", (1.into_py(py),))?)
    })?;

    println!("sleeping for 1s");
    fut.await?;
    println!("done");

    Ok(())
}
