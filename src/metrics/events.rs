use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MetricKind {
    Counter,
    Gauge,
    Histogram,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricMetadata {
    pub name: String,
    pub kind: MetricKind,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricData {
    pub name: String,
    pub labels: BTreeMap<String, String>,
    #[serde(flatten)]
    pub operation: MetricOperation,
}

/// Different metric operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", content = "value")]
#[serde(rename_all = "snake_case")]
pub enum MetricOperation {
    IncrementCounter(u64),
    SetCounter(u64),
    IncrementGauge(f64),
    DecrementGauge(f64),
    SetGauge(f64),
    RecordHistogram(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MetricEvent {
    Metadata(MetricMetadata),
    Metric(MetricData),
}
