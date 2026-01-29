use std::sync::{Arc, Mutex, atomic};

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(not(Py_GIL_DISABLED))]
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
    fn new() -> Self {
        Self {
            spawn: 0.into(),
            respawn_err: 0.into(),
            respawn_rss: 0.into(),
            respawn_ttl: 0.into(),
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

    fn format_metrics(&self) -> String {
        let prefix = "granian_";
        let mut all: Vec<String> = vec![
            format!("# TYPE {prefix}workers_spawns counter"),
            format!(
                "{prefix}workers_spawns {}",
                self.data_m.spawn.load(atomic::Ordering::Acquire)
            ),
            format!("# TYPE {prefix}workers_respawns_for_err counter"),
            format!(
                "{prefix}workers_respawns_for_err {}",
                self.data_m.respawn_err.load(atomic::Ordering::Acquire)
            ),
            format!("# TYPE {prefix}workers_respawns_for_lifetime counter"),
            format!(
                "{prefix}workers_respawns_for_lifetime {}",
                self.data_m.respawn_ttl.load(atomic::Ordering::Acquire)
            ),
            format!("# TYPE {prefix}workers_respawns_for_rss counter"),
            format!(
                "{prefix}workers_respawns_for_rss {}",
                self.data_m.respawn_rss.load(atomic::Ordering::Acquire)
            ),
        ];
        let wrk_metrics = vec![
            (format!("{prefix}worker_lifetime"), "counter"),
            (format!("{prefix}connections_active"), "gauge"),
            (format!("{prefix}connections_handled"), "counter"),
            (format!("{prefix}connections_err"), "counter"),
            (format!("{prefix}requests_handled"), "counter"),
            (format!("{prefix}static_requests_handled"), "counter"),
            (format!("{prefix}static_requests_err"), "counter"),
            (format!("{prefix}blocking_threads"), "gauge"),
            (format!("{prefix}blocking_queue"), "gauge"),
            (format!("{prefix}blocking_idle_cumulative"), "counter"),
            (format!("{prefix}blocking_busy_cumulative"), "counter"),
            (format!("{prefix}py_wait_cumulative"), "counter"),
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
            data_m: MainMetrics::new(),
            data_w: Arc::new(Mutex::new(data)),
        }
    }

    fn incr_spawn(&self, val: usize) {
        self.data_m.spawn.fetch_add(val, atomic::Ordering::Release);
    }

    fn incr_respawn_err(&self, val: usize) {
        self.data_m.respawn_err.fetch_add(val, atomic::Ordering::Release);
    }

    fn incr_respawn_ttl(&self, val: usize) {
        self.data_m.respawn_ttl.fetch_add(val, atomic::Ordering::Release);
    }

    fn incr_respawn_rss(&self, val: usize) {
        self.data_m.respawn_rss.fetch_add(val, atomic::Ordering::Release);
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct MetricsExporter {
    aggregator: Py<MetricsAggregator>,
}

impl MetricsExporter {
    pub fn tcp_listener(&self, sock: &Py<crate::net::SocketHolder>) -> std::net::TcpListener {
        let listener = sock.get().as_tcp_listener().unwrap();
        _ = listener.set_nonblocking(true);
        listener
    }

    fn data(&self) -> String {
        self.aggregator.get().format_metrics()
    }
}

#[pymethods]
impl MetricsExporter {
    #[new]
    fn new(aggregator: Py<MetricsAggregator>) -> Self {
        Self { aggregator }
    }

    fn run(pyself: Py<Self>, py: Python, sock: Py<crate::net::SocketHolder>, sig: Py<crate::workers::WorkerSignal>) {
        let sig = sig.get().rx.lock().unwrap().take().unwrap();
        let pynone = py.None().into_any().into();
        let exp = pyself.clone_ref(py);

        std::thread::spawn(move || {
            let rt = crate::runtime::init_runtime_st(1, 0, 0, pynone, None);
            let local = tokio::task::LocalSet::new();

            crate::runtime::block_on_local(&rt, local, spawn_exporter(rt.handler(), exp, sock, sig));
            Python::attach(|_| drop(rt));
        });
    }
}

#[cfg(Py_GIL_DISABLED)]
struct LocalMetricsCollector {
    data: Arc<WorkerMetrics>,
    birth: std::time::Instant,
    interval: std::time::Duration,
    id: usize,
}

#[cfg(not(Py_GIL_DISABLED))]
struct IPCMetricsCollector {
    data: Arc<WorkerMetrics>,
    birth: std::time::Instant,
    interval: std::time::Duration,
    rt: runtime::RuntimeRef,
}

#[inline(always)]
fn collect_metrics(birth: &std::time::Instant, data: &Arc<WorkerMetrics>) -> MetricsData {
    vec![
        MetricValue::Abs(birth.elapsed().as_secs() as usize),
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

#[cfg(Py_GIL_DISABLED)]
impl LocalMetricsCollector {
    fn poll(&self, aggregator: &Py<MetricsAggregator>) {
        let data = collect_metrics(&self.birth, &self.data);
        aggregator.get().collect(self.id, data);
    }
}

#[cfg(not(Py_GIL_DISABLED))]
impl IPCMetricsCollector {
    fn poll(&self, ipc: Arc<tokio::sync::Mutex<interprocess::unnamed_pipe::tokio::Sender>>) {
        self.send(collect_metrics(&self.birth, &self.data), ipc);
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
        birth: std::time::Instant::now(),
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
        birth: std::time::Instant::now(),
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

fn spawn_exporter(
    runtime: runtime::RuntimeRef,
    exporter: Py<MetricsExporter>,
    sock: Py<crate::net::SocketHolder>,
    mut sig: tokio::sync::watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    use runtime::Runtime;

    let listener_std = exporter.get().tcp_listener(&sock);
    let exp = Arc::new(exporter);
    let rth = runtime.clone();

    runtime.spawn(async move {
        let listener = tokio::net::TcpListener::from_std(listener_std).unwrap();
        loop {
            tokio::select! {
                biased;
                event = listener.accept() => {
                    match event {
                        Ok((stream, _)) => {
                            let io = hyper_util::rt::TokioIo::new(stream);
                            let exp = exp.clone();

                            rth.spawn(async move {
                                let exp = exp.clone();
                                let mut connb = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
                                connb
                                    .http1()
                                    .timer(hyper_util::rt::tokio::TokioTimer::new())
                                    .keep_alive(true)
                                    .header_read_timeout(Some(std::time::Duration::from_secs(5)));
                                connb
                                    .http2()
                                    .timer(hyper_util::rt::tokio::TokioTimer::new())
                                    .keep_alive_interval(None)
                                    .keep_alive_timeout(core::time::Duration::from_secs(20));
                                _ = connb.serve_connection(io, hyper::service::service_fn(move |_req| {
                                    let data = exp.get().data();
                                    async move {
                                        Ok::<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, std::convert::Infallible>(
                                            hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from(data)))
                                        )
                                    }
                                })).await;
                            });
                        },
                        Err(err) => {
                            log::debug!("Metrics exporter TCP handshake failed with error: {err:?}");
                        },
                    }
                },
                _ = sig.changed() => break,
            }
        }

        Python::attach(|_| {
            drop(sock);
            drop(exp);
            drop(rth);
        });
    })
}

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<MetricsAggregator>()?;
    module.add_class::<MetricsExporter>()?;

    Ok(())
}
