use std::sync::{Arc, Mutex, atomic};

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ipc;
use crate::runtime;

pub(crate) type MetricsData = Vec<MetricValue>;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum MetricValue {
    Abs(usize),
    Int(isize),
}

impl std::fmt::Display for MetricValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Abs(v) => v.fmt(f),
            Self::Int(v) => v.fmt(f),
        }
    }
}

struct MainMetrics {
    spawn: atomic::AtomicUsize,
    respawn_err: atomic::AtomicUsize,
    respawn_ttl: atomic::AtomicUsize,
    respawn_rss: atomic::AtomicUsize,
    workers_life: Arc<Mutex<Vec<usize>>>,
}

pub(crate) struct WorkerMetrics {
    pub conn_active: atomic::AtomicUsize,
    pub conn_handled: atomic::AtomicUsize,
    pub conn_err: atomic::AtomicUsize,
    pub req_handled: atomic::AtomicUsize,
    pub req_static_handled: atomic::AtomicUsize,
    pub req_static_err: atomic::AtomicUsize,
    pub blocking_threads: atomic::AtomicUsize,
    pub blocking_queue: atomic::AtomicIsize,
    pub blocking_idle_cumul: atomic::AtomicUsize,
    pub blocking_busy_cumul: atomic::AtomicUsize,
    pub py_wait_cumul: atomic::AtomicUsize,
}

impl MainMetrics {
    fn new(workers: usize) -> Self {
        Self {
            spawn: 0.into(),
            respawn_err: 0.into(),
            respawn_rss: 0.into(),
            respawn_ttl: 0.into(),
            workers_life: Arc::new(Mutex::new(vec![0, workers])),
        }
    }
}

impl WorkerMetrics {
    pub fn new() -> Self {
        Self {
            conn_active: 0.into(),
            conn_handled: 0.into(),
            conn_err: 0.into(),
            req_handled: 0.into(),
            req_static_handled: 0.into(),
            req_static_err: 0.into(),
            blocking_threads: 0.into(),
            blocking_queue: 0.into(),
            blocking_idle_cumul: 0.into(),
            blocking_busy_cumul: 0.into(),
            py_wait_cumul: 0.into(),
        }
    }
}

pub(crate) type ArcWorkerMetrics = Arc<WorkerMetrics>;

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct MetricsAggregator {
    data_m: MainMetrics,
    data_w: Arc<Mutex<Vec<MetricsData>>>,
}

impl MetricsAggregator {
    pub fn collect(&self, id: usize, data: MetricsData) {
        let mut aggr = self.data_w.lock().unwrap();
        aggr[id] = data;
    }

    #[allow(dead_code)]
    fn format_metrics(&self) -> String {
        let mut all: Vec<String> = vec![
            "# TYPE granian_workers_spawns counter".into(),
            format!(
                "granian_workers_spawns {}",
                self.data_m.spawn.load(atomic::Ordering::Acquire)
            ),
            "# TYPE granian_workers_respawns_due_err counter".into(),
            format!(
                "granian_workers_spawns {}",
                self.data_m.respawn_err.load(atomic::Ordering::Acquire)
            ),
            "# TYPE granian_workers_respawns_due_lifetime counter".into(),
            format!(
                "granian_workers_spawns {}",
                self.data_m.respawn_ttl.load(atomic::Ordering::Acquire)
            ),
            "# TYPE granian_workers_respawns_due_rss counter".into(),
            format!(
                "granian_workers_spawns {}",
                self.data_m.respawn_rss.load(atomic::Ordering::Acquire)
            ),
        ];
        let mut mwrk: Vec<String> = vec!["# TYPE granian_worker_lifetime counter".into()];
        {
            let wrk_lifetimes = self.data_m.workers_life.lock().unwrap();
            for (idx, item) in wrk_lifetimes.iter().enumerate() {
                mwrk.push(format!("granian_worker_lifetime{{worker=\"{}\"}} {}", idx + 1, item));
            }
        }
        all.append(&mut mwrk);
        let wrk_metrics = vec![
            ("granian_connections_active", "gauge"),
            ("granian_connections_handled", "counter"),
            ("granian_connections_err", "counter"),
            ("granian_requests_handled", "counter"),
            ("granian_static_requests_handled", "counter"),
            ("granian_static_requests_err", "counter"),
            ("granian_blocking_threads", "gauge"),
            ("granian_blocking_queue", "gauge"),
            ("granian_blocking_idle_cumulative", "counter"),
            ("granian_blocking_busy_cumulative", "counter"),
            ("granian_py_wait_cumulative", "counter"),
        ];
        let mut wrk: Vec<Vec<String>> = vec![vec![]; wrk_metrics.len()];
        {
            let wrk_data = self.data_w.lock().unwrap();
            for (metric_idx, (metric_label, metric_type)) in wrk_metrics.iter().enumerate() {
                wrk[metric_idx].push(format!("# TYPE {metric_label} {metric_type}"));
                for (idx, values) in wrk_data.iter().enumerate() {
                    if let Some(value) = values.get(metric_idx) {
                        wrk[metric_idx].push(format!("{}{{worker=\"{}\"}} {}", metric_label, idx + 1, value));
                    }
                }
            }
        }
        for items in &mut wrk {
            all.append(items);
        }
        all.join("\n")
    }
}

#[pymethods]
impl MetricsAggregator {
    #[new]
    fn new(w_size: usize) -> Self {
        let mut data = Vec::with_capacity(w_size);
        for _ in 0..w_size {
            data.push(Vec::new());
        }
        Self {
            data_m: MainMetrics::new(w_size),
            data_w: Arc::new(Mutex::new(data)),
        }
    }

    fn update_main(
        &self,
        spawn: usize,
        respawn_err: usize,
        respawn_ttl: usize,
        respawn_rss: usize,
        workers_life: Vec<usize>,
    ) {
        self.data_m.spawn.store(spawn, atomic::Ordering::Release);
        self.data_m.respawn_err.store(respawn_err, atomic::Ordering::Release);
        self.data_m.respawn_ttl.store(respawn_ttl, atomic::Ordering::Release);
        self.data_m.respawn_rss.store(respawn_rss, atomic::Ordering::Release);
        let mut wrk_life = self.data_m.workers_life.lock().unwrap();
        *wrk_life = workers_life;
    }
}

#[allow(dead_code)]
struct LocalMetricsCollector {
    data: Arc<WorkerMetrics>,
    interval: std::time::Duration,
    id: usize,
}

struct IPCMetricsCollector {
    data: Arc<WorkerMetrics>,
    interval: std::time::Duration,
    rt: runtime::RuntimeRef,
}

#[inline(always)]
fn collect_metrics(data: &Arc<WorkerMetrics>) -> MetricsData {
    vec![
        MetricValue::Abs(data.conn_active.load(atomic::Ordering::Acquire)),
        MetricValue::Abs(data.conn_handled.load(atomic::Ordering::Acquire)),
        MetricValue::Abs(data.conn_err.load(atomic::Ordering::Acquire)),
        MetricValue::Abs(data.req_handled.load(atomic::Ordering::Acquire)),
        MetricValue::Abs(data.req_static_handled.load(atomic::Ordering::Acquire)),
        MetricValue::Abs(data.req_static_err.load(atomic::Ordering::Acquire)),
        MetricValue::Abs(data.blocking_threads.load(atomic::Ordering::Acquire)),
        MetricValue::Int(data.blocking_queue.load(atomic::Ordering::Acquire)),
        MetricValue::Abs(data.blocking_idle_cumul.load(atomic::Ordering::Acquire)),
        MetricValue::Abs(data.blocking_busy_cumul.load(atomic::Ordering::Acquire)),
        MetricValue::Abs(data.py_wait_cumul.load(atomic::Ordering::Acquire)),
    ]
}

impl LocalMetricsCollector {
    #[allow(dead_code)]
    fn poll(&self, aggregator: &Py<MetricsAggregator>) {
        let data = collect_metrics(&self.data);
        aggregator.get().collect(self.id, data);
    }
}

impl IPCMetricsCollector {
    fn poll(&self, ipc: Arc<tokio::sync::Mutex<interprocess::unnamed_pipe::tokio::Sender>>) {
        self.send(collect_metrics(&self.data), ipc);
    }

    fn send(&self, data: MetricsData, ipc: Arc<tokio::sync::Mutex<interprocess::unnamed_pipe::tokio::Sender>>) {
        use runtime::Runtime;

        self.rt.spawn(async move {
            let mut sender = ipc.lock().await;
            _ = ipc::write_msg(ipc::Message::Metrics(data), &mut *sender).await;
        });
    }
}

#[cfg(not(Py_GIL_DISABLED))]
pub(crate) fn spawn_ipc_collector(
    runtime: runtime::RuntimeRef,
    mut sig: tokio::sync::watch::Receiver<bool>,
    notify: Arc<tokio::sync::Notify>,
    metrics: ArcWorkerMetrics,
    interval: std::time::Duration,
    ipc: Py<ipc::IPCSenderHandle>,
) {
    use runtime::Runtime;

    let collector = IPCMetricsCollector {
        data: metrics,
        interval,
        rt: runtime.clone(),
    };
    runtime.spawn(async move {
        let sender = ipc.get().to_sender();
        loop {
            let sender = sender.clone();
            tokio::select! {
                biased;
                () = tokio::time::sleep(collector.interval) => {
                    collector.poll(sender);
                },
                _ = sig.changed() => break,
            }
        }
        Python::attach(|_| drop(ipc));
        notify.notify_one();
    });
}

#[cfg(Py_GIL_DISABLED)]
pub(crate) fn spawn_local_collector(
    runtime: runtime::RuntimeRef,
    mut sig: tokio::sync::watch::Receiver<bool>,
    notify: Arc<tokio::sync::Notify>,
    metrics: ArcWorkerMetrics,
    interval: std::time::Duration,
    idx: usize,
    aggregator: Py<MetricsAggregator>,
) {
    use runtime::Runtime;

    let collector = LocalMetricsCollector {
        id: idx,
        data: metrics,
        interval,
    };
    runtime.spawn(async move {
        loop {
            tokio::select! {
                biased;
                () = tokio::time::sleep(collector.interval) => {
                    collector.poll(&aggregator);
                },
                _ = sig.changed() => break,
            }
        }
        Python::attach(|_| drop(aggregator));
        notify.notify_one();
    });
}

// TODO: web scraper listener

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<MetricsAggregator>()?;

    Ok(())
}
