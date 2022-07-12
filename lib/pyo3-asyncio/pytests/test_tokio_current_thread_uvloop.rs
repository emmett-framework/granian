#[cfg(not(target_os = "windows"))]
fn main() -> pyo3::PyResult<()> {
    use pyo3::{prelude::*, types::PyType};

    pyo3::prepare_freethreaded_python();

    let mut builder = tokio::runtime::Builder::new_current_thread();
    builder.enable_all();

    pyo3_asyncio::tokio::init(builder);
    std::thread::spawn(move || {
        pyo3_asyncio::tokio::get_runtime().block_on(futures::future::pending::<()>());
    });

    Python::with_gil(|py| {
        let uvloop = py.import("uvloop")?;
        uvloop.call_method0("install")?;

        // store a reference for the assertion
        let uvloop = PyObject::from(uvloop);

        pyo3_asyncio::tokio::run(py, async move {
            // verify that we are on a uvloop.Loop
            Python::with_gil(|py| -> PyResult<()> {
                assert!(pyo3_asyncio::tokio::get_current_loop(py)?.is_instance(
                    uvloop
                        .as_ref(py)
                        .getattr("Loop")?
                        .downcast::<PyType>()
                        .unwrap()
                )?);
                Ok(())
            })?;

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            println!("test test_tokio_current_thread_uvloop ... ok");
            Ok(())
        })
    })
}

#[cfg(target_os = "windows")]
fn main() {}
