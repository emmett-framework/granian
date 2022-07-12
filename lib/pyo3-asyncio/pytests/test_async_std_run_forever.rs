use std::time::Duration;

use pyo3::prelude::*;

fn dump_err(py: Python, e: PyErr) {
    // We can't display Python exceptions via std::fmt::Display,
    // so print the error here manually.
    e.print_and_set_sys_last_vars(py);
}

fn main() {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| {
        let asyncio = py.import("asyncio")?;

        let event_loop = asyncio.call_method0("new_event_loop")?;
        asyncio.call_method1("set_event_loop", (event_loop,))?;

        let event_loop_hdl = PyObject::from(event_loop);

        async_std::task::spawn(async move {
            async_std::task::sleep(Duration::from_secs(1)).await;

            Python::with_gil(|py| {
                event_loop_hdl
                    .as_ref(py)
                    .call_method1(
                        "call_soon_threadsafe",
                        (event_loop_hdl
                            .as_ref(py)
                            .getattr("stop")
                            .map_err(|e| dump_err(py, e))
                            .unwrap(),),
                    )
                    .map_err(|e| dump_err(py, e))
                    .unwrap();
            })
        });

        event_loop.call_method0("run_forever")?;

        println!("test test_async_std_run_forever ... ok");
        Ok(())
    })
    .map_err(|e| Python::with_gil(|py| dump_err(py, e)))
    .unwrap()
}
