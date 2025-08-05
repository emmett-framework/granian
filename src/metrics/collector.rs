use super::{
    error::MetricsError,
    events::{MetricData, MetricEvent, MetricKind, MetricMetadata, MetricOperation},
};
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, ListenerNonblockingMode, ListenerOptions, prelude::*,
};
use metrics_exporter_prometheus::PrometheusBuilder;
use serde_json;
use std::{
    io::{BufRead, BufReader},
    net::SocketAddr,
    path::PathBuf,
    thread,
};

pub struct MetricsCollector {
    socket_path: PathBuf,
    address: SocketAddr,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self {
            socket_path: "/tmp/metrics_collector.sock".into(),
            address: SocketAddr::from(([127, 0, 0, 1], 9000)),
        }
    }
}

impl MetricsCollector {
    pub fn address(mut self, address: SocketAddr) -> Self {
        self.address = address;
        self
    }

    pub fn start_collecting(self) -> Result<(), MetricsError> {
        PrometheusBuilder::new().with_http_listener(self.address).install()?;

        let socket_path = self.socket_path.clone();

        thread::spawn(move || {
            if let Err(e) = run_collector(socket_path.clone()) {
                log::error!("Metrics collector error: {}", e);
            }
            // Clean up socket file on shutdown
            let _ = std::fs::remove_file(&socket_path);
        });

        Ok(())
    }
}

fn run_collector(socket_path: PathBuf) -> Result<(), MetricsError> {
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }

    let socket_name = if GenericNamespaced::is_supported() {
        "metrics_collector.sock".to_ns_name::<GenericNamespaced>()?
    } else {
        socket_path.clone().to_fs_name::<GenericFilePath>()?
    };

    let listener = ListenerOptions::new().name(socket_name).create_sync()?;
    listener.set_nonblocking(ListenerNonblockingMode::Both)?;

    for conn in listener.incoming() {
        if let Ok(stream) = conn {
            thread::spawn(move || {
                let reader = BufReader::new(stream);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        if line.trim().is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<MetricEvent>(&line) {
                            Ok(MetricEvent::Metadata(metadata)) => handle_metadata_event(metadata),
                            Ok(MetricEvent::Metric(metric)) => handle_metric_event(metric),
                            Err(e) => {
                                log::error!("Failed to deserialize metric event: {} (line: {})", e, line);
                            }
                        }
                    }
                }
            });
        }
    }
    Ok(())
}

fn handle_metric_event(metric: MetricData) {
    match metric.operation {
        MetricOperation::IncrementCounter(value) => {
            if metric.labels.is_empty() {
                metrics::counter!(metric.name).increment(value);
            } else {
                let labels: Vec<_> = metric
                    .labels
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                metrics::counter!(metric.name, &labels).increment(value);
            }
        }
        MetricOperation::SetCounter(value) => {
            if metric.labels.is_empty() {
                metrics::counter!(metric.name).absolute(value);
            } else {
                let labels: Vec<_> = metric
                    .labels
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                metrics::counter!(metric.name, &labels).absolute(value);
            }
        }
        MetricOperation::IncrementGauge(value) => {
            if metric.labels.is_empty() {
                metrics::gauge!(metric.name).increment(value);
            } else {
                let labels: Vec<_> = metric
                    .labels
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                metrics::gauge!(metric.name, &labels).increment(value);
            }
        }
        MetricOperation::DecrementGauge(value) => {
            if metric.labels.is_empty() {
                metrics::gauge!(metric.name).decrement(value);
            } else {
                let labels: Vec<_> = metric
                    .labels
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                metrics::gauge!(metric.name, &labels).decrement(value);
            }
        }
        MetricOperation::SetGauge(value) => {
            if metric.labels.is_empty() {
                metrics::gauge!(metric.name).set(value);
            } else {
                let labels: Vec<_> = metric
                    .labels
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                metrics::gauge!(metric.name, &labels).set(value);
            }
        }
        MetricOperation::RecordHistogram(value) => {
            if metric.labels.is_empty() {
                metrics::histogram!(metric.name).record(value);
            } else {
                let labels: Vec<_> = metric
                    .labels
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                metrics::histogram!(metric.name, &labels).record(value);
            }
        }
    }
}

fn handle_metadata_event(metadata: MetricMetadata) {
    let unit = if let Some(ref unit) = metadata.unit {
        metrics::Unit::from_string(unit)
    } else {
        None
    };

    match metadata.kind {
        MetricKind::Counter => {
            if let Some(unit) = unit {
                metrics::describe_counter!(metadata.name, unit, metadata.description);
            } else {
                metrics::describe_counter!(metadata.name, metadata.description);
            }
        }
        MetricKind::Gauge => {
            if let Some(unit) = unit {
                metrics::describe_gauge!(metadata.name, unit, metadata.description);
            } else {
                metrics::describe_gauge!(metadata.name, metadata.description);
            }
        }
        MetricKind::Histogram => {
            if let Some(unit) = unit {
                metrics::describe_histogram!(metadata.name, unit, metadata.description);
            } else {
                metrics::describe_histogram!(metadata.name, metadata.description);
            }
        }
    }
}
