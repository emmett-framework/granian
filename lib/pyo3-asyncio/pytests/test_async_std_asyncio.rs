mod common;

use std::{
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_std::task;
use pyo3::{
    prelude::*,
    types::{IntoPyDict, PyType},
    wrap_pyfunction, wrap_pymodule,
};
use pyo3_asyncio::TaskLocals;

#[cfg(feature = "unstable-streams")]
use futures::{StreamExt, TryStreamExt};

#[pyfunction]
fn sleep<'p>(py: Python<'p>, secs: &'p PyAny) -> PyResult<&'p PyAny> {
    let secs = secs.extract()?;

    pyo3_asyncio::async_std::future_into_py(py, async move {
        task::sleep(Duration::from_secs(secs)).await;
        Ok(())
    })
}

#[pyo3_asyncio::async_std::test]
async fn test_future_into_py() -> PyResult<()> {
    let fut = Python::with_gil(|py| {
        let sleeper_mod = PyModule::new(py, "rust_sleeper")?;

        sleeper_mod.add_wrapped(wrap_pyfunction!(sleep))?;

        let test_mod = PyModule::from_code(
            py,
            common::TEST_MOD,
            "test_future_into_py_mod.py",
            "test_future_into_py_mod",
        )?;

        pyo3_asyncio::async_std::into_future(
            test_mod.call_method1("sleep_for_1s", (sleeper_mod.getattr("sleep")?,))?,
        )
    })?;

    fut.await?;

    Ok(())
}

#[pyo3_asyncio::async_std::test]
async fn test_async_sleep() -> PyResult<()> {
    let asyncio =
        Python::with_gil(|py| py.import("asyncio").map(|asyncio| PyObject::from(asyncio)))?;

    task::sleep(Duration::from_secs(1)).await;

    Python::with_gil(|py| {
        pyo3_asyncio::async_std::into_future(asyncio.as_ref(py).call_method1("sleep", (1.0,))?)
    })?
    .await?;

    Ok(())
}

#[pyo3_asyncio::async_std::test]
fn test_blocking_sleep() -> PyResult<()> {
    common::test_blocking_sleep()
}

#[pyo3_asyncio::async_std::test]
async fn test_into_future() -> PyResult<()> {
    common::test_into_future(Python::with_gil(|py| {
        pyo3_asyncio::async_std::get_current_loop(py)
            .unwrap()
            .into()
    }))
    .await
}

#[pyo3_asyncio::async_std::test]
async fn test_other_awaitables() -> PyResult<()> {
    common::test_other_awaitables(Python::with_gil(|py| {
        pyo3_asyncio::async_std::get_current_loop(py)
            .unwrap()
            .into()
    }))
    .await
}

#[pyo3_asyncio::async_std::test]
async fn test_panic() -> PyResult<()> {
    let fut = Python::with_gil(|py| -> PyResult<_> {
        pyo3_asyncio::async_std::into_future(pyo3_asyncio::async_std::future_into_py::<_, ()>(
            py,
            async { panic!("this panic was intentional!") },
        )?)
    })?;

    match fut.await {
        Ok(_) => panic!("coroutine should panic"),
        Err(e) => Python::with_gil(|py| {
            if e.is_instance_of::<pyo3_asyncio::err::RustPanic>(py) {
                Ok(())
            } else {
                panic!("expected RustPanic err")
            }
        }),
    }
}

#[pyo3_asyncio::async_std::test]
async fn test_local_future_into_py() -> PyResult<()> {
    Python::with_gil(|py| {
        let non_send_secs = Rc::new(1);

        let py_future = pyo3_asyncio::async_std::local_future_into_py(py, async move {
            async_std::task::sleep(Duration::from_secs(*non_send_secs)).await;
            Ok(())
        })?;

        pyo3_asyncio::async_std::into_future(py_future)
    })?
    .await?;

    Ok(())
}

#[pyo3_asyncio::async_std::test]
async fn test_cancel() -> PyResult<()> {
    let completed = Arc::new(Mutex::new(false));

    let py_future = Python::with_gil(|py| -> PyResult<PyObject> {
        let completed = Arc::clone(&completed);
        Ok(pyo3_asyncio::async_std::future_into_py(py, async move {
            async_std::task::sleep(Duration::from_secs(1)).await;
            *completed.lock().unwrap() = true;

            Ok(())
        })?
        .into())
    })?;

    if let Err(e) = Python::with_gil(|py| -> PyResult<_> {
        py_future.as_ref(py).call_method0("cancel")?;
        pyo3_asyncio::async_std::into_future(py_future.as_ref(py))
    })?
    .await
    {
        Python::with_gil(|py| -> PyResult<()> {
            assert!(e.value(py).is_instance(
                py.import("asyncio")?
                    .getattr("CancelledError")?
                    .downcast::<PyType>()
                    .unwrap()
            )?);
            Ok(())
        })?;
    } else {
        panic!("expected CancelledError");
    }

    async_std::task::sleep(Duration::from_secs(1)).await;
    if *completed.lock().unwrap() {
        panic!("future still completed")
    }

    Ok(())
}

#[cfg(feature = "unstable-streams")]
const ASYNC_STD_TEST_MOD: &str = r#"
import asyncio

async def gen():
    for i in range(10):
        await asyncio.sleep(0.1)
        yield i
"#;

#[cfg(feature = "unstable-streams")]
#[pyo3_asyncio::async_std::test]
async fn test_async_gen_v1() -> PyResult<()> {
    let stream = Python::with_gil(|py| {
        let test_mod = PyModule::from_code(
            py,
            ASYNC_STD_TEST_MOD,
            "test_rust_coroutine/async_std_test_mod.py",
            "async_std_test_mod",
        )?;

        pyo3_asyncio::async_std::into_stream_v1(test_mod.call_method0("gen")?)
    })?;

    let vals = stream
        .map(|item| Python::with_gil(|py| -> PyResult<i32> { Ok(item?.as_ref(py).extract()?) }))
        .try_collect::<Vec<i32>>()
        .await?;

    assert_eq!((0..10).collect::<Vec<i32>>(), vals);

    Ok(())
}

#[pyo3_asyncio::async_std::test]
fn test_local_cancel(event_loop: PyObject) -> PyResult<()> {
    let locals = Python::with_gil(|py| -> PyResult<TaskLocals> {
        Ok(TaskLocals::new(event_loop.as_ref(py)).copy_context(py)?)
    })?;
    async_std::task::block_on(pyo3_asyncio::async_std::scope_local(locals, async {
        let completed = Arc::new(Mutex::new(false));

        let py_future = Python::with_gil(|py| -> PyResult<PyObject> {
            let completed = Arc::clone(&completed);
            Ok(pyo3_asyncio::async_std::future_into_py(py, async move {
                async_std::task::sleep(Duration::from_secs(1)).await;
                *completed.lock().unwrap() = true;

                Ok(())
            })?
            .into())
        })?;

        if let Err(e) = Python::with_gil(|py| -> PyResult<_> {
            py_future.as_ref(py).call_method0("cancel")?;
            pyo3_asyncio::async_std::into_future(py_future.as_ref(py))
        })?
        .await
        {
            Python::with_gil(|py| -> PyResult<()> {
                assert!(e.value(py).is_instance(
                    py.import("asyncio")?
                        .getattr("CancelledError")?
                        .downcast::<PyType>()
                        .unwrap()
                )?);
                Ok(())
            })?;
        } else {
            panic!("expected CancelledError");
        }

        async_std::task::sleep(Duration::from_secs(1)).await;
        if *completed.lock().unwrap() {
            panic!("future still completed")
        }

        Ok(())
    }))
}

/// This module is implemented in Rust.
#[pymodule]
fn test_mod(_py: Python, m: &PyModule) -> PyResult<()> {
    #![allow(deprecated)]
    #[pyfunction(name = "sleep")]
    fn sleep_(py: Python) -> PyResult<&PyAny> {
        pyo3_asyncio::async_std::future_into_py(py, async move {
            async_std::task::sleep(Duration::from_millis(500)).await;
            Ok(())
        })
    }

    m.add_function(wrap_pyfunction!(sleep_, m)?)?;

    Ok(())
}

const MULTI_ASYNCIO_CODE: &str = r#"
async def main():
    return await test_mod.sleep()

asyncio.new_event_loop().run_until_complete(main())
"#;

#[pyo3_asyncio::async_std::test]
fn test_multiple_asyncio_run() -> PyResult<()> {
    Python::with_gil(|py| {
        pyo3_asyncio::async_std::run(py, async move {
            async_std::task::sleep(Duration::from_millis(500)).await;
            Ok(())
        })?;
        pyo3_asyncio::async_std::run(py, async move {
            async_std::task::sleep(Duration::from_millis(500)).await;
            Ok(())
        })?;

        let d = [
            ("asyncio", py.import("asyncio")?.into()),
            ("test_mod", wrap_pymodule!(test_mod)(py)),
        ]
        .into_py_dict(py);

        py.run(MULTI_ASYNCIO_CODE, Some(d), None)?;
        py.run(MULTI_ASYNCIO_CODE, Some(d), None)?;
        Ok(())
    })
}

#[pymodule]
fn cvars_mod(_py: Python, m: &PyModule) -> PyResult<()> {
    #![allow(deprecated)]
    #[pyfunction]
    pub(crate) fn async_callback(py: Python, callback: PyObject) -> PyResult<&PyAny> {
        pyo3_asyncio::async_std::future_into_py(py, async move {
            Python::with_gil(|py| {
                pyo3_asyncio::async_std::into_future(callback.as_ref(py).call0()?)
            })?
            .await?;

            Ok(())
        })
    }

    m.add_function(wrap_pyfunction!(async_callback, m)?)?;

    Ok(())
}

#[cfg(feature = "unstable-streams")]
#[pyo3_asyncio::async_std::test]
async fn test_async_gen_v2() -> PyResult<()> {
    let stream = Python::with_gil(|py| {
        let test_mod = PyModule::from_code(
            py,
            ASYNC_STD_TEST_MOD,
            "test_rust_coroutine/async_std_test_mod.py",
            "async_std_test_mod",
        )?;

        pyo3_asyncio::async_std::into_stream_v2(test_mod.call_method0("gen")?)
    })?;

    let vals = stream
        .map(|item| Python::with_gil(|py| -> PyResult<i32> { Ok(item.as_ref(py).extract()?) }))
        .try_collect::<Vec<i32>>()
        .await?;

    assert_eq!((0..10).collect::<Vec<i32>>(), vals);

    Ok(())
}

const CONTEXTVARS_CODE: &str = r#"
cx = contextvars.ContextVar("cx")

async def contextvars_test():
    assert cx.get() == "foobar"

async def main():
    cx.set("foobar")
    await cvars_mod.async_callback(contextvars_test)

asyncio.run(main())
"#;

#[pyo3_asyncio::async_std::test]
fn test_contextvars() -> PyResult<()> {
    Python::with_gil(|py| {
        let d = [
            ("asyncio", py.import("asyncio")?.into()),
            ("contextvars", py.import("contextvars")?.into()),
            ("cvars_mod", wrap_pymodule!(cvars_mod)(py)),
        ]
        .into_py_dict(py);

        py.run(CONTEXTVARS_CODE, Some(d), None)?;
        py.run(CONTEXTVARS_CODE, Some(d), None)?;
        Ok(())
    })
}

fn main() -> pyo3::PyResult<()> {
    pyo3::prepare_freethreaded_python();

    Python::with_gil(|py| pyo3_asyncio::async_std::run(py, pyo3_asyncio::testing::main()))
}
