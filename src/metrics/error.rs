use super::recorder::IPCRecorder;

#[derive(Debug)]
pub enum MetricsError {
    PrometheusError(metrics_exporter_prometheus::BuildError),
    Io(std::io::Error),
    Recorder(metrics::SetRecorderError<IPCRecorder>),
    Serialization(serde_json::Error),
}

impl std::fmt::Display for MetricsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricsError::PrometheusError(err) => {
                write!(f, "failed to create prometheus exporter {}", err)
            }
            MetricsError::Io(err) => {
                write!(f, "IO error setting up metrics reporter {}", err)
            }
            MetricsError::Recorder(err) => {
                write!(f, "failed to set IPCRecorder: {}", err)
            }
            MetricsError::Serialization(err) => {
                write!(f, "couldnt serialize event: {}", err)
            }
        }
    }
}

impl std::error::Error for MetricsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MetricsError::PrometheusError(err) => Some(err),
            MetricsError::Io(err) => Some(err),
            MetricsError::Recorder(err) => Some(err),
            MetricsError::Serialization(err) => Some(err),
        }
    }
}

impl From<metrics_exporter_prometheus::BuildError> for MetricsError {
    fn from(err: metrics_exporter_prometheus::BuildError) -> Self {
        MetricsError::PrometheusError(err)
    }
}

impl From<std::io::Error> for MetricsError {
    fn from(err: std::io::Error) -> Self {
        MetricsError::Io(err)
    }
}

impl From<metrics::SetRecorderError<IPCRecorder>> for MetricsError {
    fn from(err: metrics::SetRecorderError<IPCRecorder>) -> Self {
        MetricsError::Recorder(err)
    }
}

impl From<serde_json::Error> for MetricsError {
    fn from(err: serde_json::Error) -> Self {
        MetricsError::Serialization(err)
    }
}
