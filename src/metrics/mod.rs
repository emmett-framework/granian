use pyo3::prelude::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub mod collector;
pub mod error;
pub mod events;
pub mod recorder;

#[pyfunction]
fn init_metrics(addr: &str, port: i32) {
    _ = pyo3_log::try_init();

    let address: SocketAddr = format!("{addr}:{port}")
        .parse()
        .unwrap_or_else(|_| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000));

    if let Err(e) = collector::MetricsCollector::default()
        .address(address)
        .start_collecting()
    {
        log::warn!("Failed to start metrics collector process {e}");
    }

    metrics::describe_gauge!(
        "granian_number_workers",
        "Current number of active workers in the Granian application"
    );
    metrics::describe_counter!(
        "granian_worker_started_total",
        "Total number of workers started since the application started"
    );
    metrics::describe_counter!(
        "granian_worker_stopped_total",
        "Total number of workers stopped since the application started"
    );
    metrics::describe_counter!(
        "granian_requests_processed_total",
        "Total count of HTTP requests processed by each worker since startup"
    );
    metrics::describe_histogram!(
        "granian_python_call_latency",
        metrics::Unit::Seconds,
        "Time spent executing Python application code within each worker, measured in seconds"
    );
    metrics::describe_counter!(
        "granian_http_responses_total",
        "Counter of HTTP responses grouped by status code per worker",
    );
    metrics::describe_counter!(
        "granian_requests_processed_total",
        "Total count of HTTP requests processed by each worker since startup"
    );
    metrics::describe_counter!(
        "granian_connections_total",
        "Total number of connections established by each worker since startup"
    );
    metrics::describe_counter!(
        "granian_connection_errors_total",
        "Total number of connection errors encountered by each worker since startup"
    );
}

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(init_metrics, module)?)?;
    Ok(())
}
