use super::{error::MetricsError, events::*};
use interprocess::local_socket::{GenericFilePath, GenericNamespaced, prelude::*};
use std::{
    collections::BTreeMap,
    io::{self, Write},
    sync::{Arc, Mutex},
};

fn write_event(stream: Arc<Mutex<LocalSocketStream>>, event: &MetricEvent) -> Result<(), io::Error> {
    let json_bytes = serde_json::to_vec(event).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut stream = stream.lock().unwrap();
    stream.write_all(&json_bytes)?;
    stream.write_all(b"\n")?;
    stream.flush()
}

#[derive(Debug)]
struct Handle {
    key: metrics::Key,
    stream: Arc<Mutex<LocalSocketStream>>,
}

impl Handle {
    fn new(key: metrics::Key, stream: Arc<Mutex<LocalSocketStream>>) -> Self {
        Self { key, stream }
    }

    fn push_metric(&self, key: &metrics::Key, op: MetricOperation) {
        let metric = MetricData {
            name: key.name().to_string(),
            labels: key
                .labels()
                .map(|label| (label.key().to_owned(), label.value().to_owned()))
                .collect::<BTreeMap<_, _>>(),
            operation: op,
        };
        let _ = write_event(self.stream.clone(), &MetricEvent::Metric(metric));
    }
}

impl metrics::CounterFn for Handle {
    fn increment(&self, value: u64) {
        self.push_metric(&self.key, MetricOperation::IncrementCounter(value))
    }

    fn absolute(&self, value: u64) {
        self.push_metric(&self.key, MetricOperation::SetCounter(value))
    }
}

impl metrics::GaugeFn for Handle {
    fn increment(&self, value: f64) {
        self.push_metric(&self.key, MetricOperation::IncrementGauge(value))
    }

    fn decrement(&self, value: f64) {
        self.push_metric(&self.key, MetricOperation::DecrementGauge(value))
    }

    fn set(&self, value: f64) {
        self.push_metric(&self.key, MetricOperation::SetGauge(value))
    }
}

impl metrics::HistogramFn for Handle {
    fn record(&self, value: f64) {
        self.push_metric(&self.key, MetricOperation::RecordHistogram(value))
    }
}

/// An IPC recorder.
#[derive(Debug, Clone)]
pub struct IPCRecorder {
    stream: Arc<Mutex<LocalSocketStream>>,
}

impl IPCRecorder {
    pub fn new(stream: LocalSocketStream) -> Self {
        Self {
            stream: Arc::new(Mutex::new(stream)),
        }
    }

    fn register_metric(
        &self,
        key_name: metrics::KeyName,
        kind: MetricKind,
        unit: Option<metrics::Unit>,
        description: metrics::SharedString,
    ) {
        let metadata = MetricMetadata {
            name: key_name.as_str().to_string(),
            kind,
            unit: unit.map(|u| u.as_str().to_string()),
            description: description.to_string(),
        };
        let _ = write_event(self.stream.clone(), &MetricEvent::Metadata(metadata));
    }
}

impl metrics::Recorder for IPCRecorder {
    fn describe_counter(
        &self,
        key_name: metrics::KeyName,
        unit: Option<metrics::Unit>,
        description: metrics::SharedString,
    ) {
        self.register_metric(key_name, MetricKind::Counter, unit, description);
    }

    fn describe_gauge(
        &self,
        key_name: metrics::KeyName,
        unit: Option<metrics::Unit>,
        description: metrics::SharedString,
    ) {
        self.register_metric(key_name, MetricKind::Gauge, unit, description);
    }

    fn describe_histogram(
        &self,
        key_name: metrics::KeyName,
        unit: Option<metrics::Unit>,
        description: metrics::SharedString,
    ) {
        self.register_metric(key_name, MetricKind::Histogram, unit, description);
    }

    fn register_counter(&self, key: &metrics::Key, _meta: &metrics::Metadata<'_>) -> metrics::Counter {
        metrics::Counter::from_arc(Arc::new(Handle::new(key.clone(), self.stream.clone())))
    }

    fn register_gauge(&self, key: &metrics::Key, _meta: &metrics::Metadata<'_>) -> metrics::Gauge {
        metrics::Gauge::from_arc(Arc::new(Handle::new(key.clone(), self.stream.clone())))
    }

    fn register_histogram(&self, key: &metrics::Key, _meta: &metrics::Metadata<'_>) -> metrics::Histogram {
        metrics::Histogram::from_arc(Arc::new(Handle::new(key.clone(), self.stream.clone())))
    }
}

#[derive(Debug, Default)]
pub struct IPCBuilder {}

impl IPCBuilder {
    pub fn build(self) -> Result<(), MetricsError> {
        let socket_name = if GenericNamespaced::is_supported() {
            "metrics_collector.sock".to_ns_name::<GenericNamespaced>()?
        } else {
            "/tmp/metrics_collector.sock".to_fs_name::<GenericFilePath>()?
        };

        let stream = LocalSocketStream::connect(socket_name)?;
        stream.set_nonblocking(true)?;
        let recorder = IPCRecorder::new(stream);
        metrics::set_global_recorder(recorder).map_err(Into::into)
    }
}
