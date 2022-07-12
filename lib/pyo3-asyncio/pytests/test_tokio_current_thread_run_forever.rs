mod tokio_run_forever;

fn main() {
    pyo3::prepare_freethreaded_python();

    let mut builder = tokio::runtime::Builder::new_current_thread();
    builder.enable_all();

    pyo3_asyncio::tokio::init(builder);
    std::thread::spawn(move || {
        pyo3_asyncio::tokio::get_runtime().block_on(futures::future::pending::<()>());
    });

    tokio_run_forever::test_main();
    println!("test test_tokio_current_thread_run_forever ... ok");
}
