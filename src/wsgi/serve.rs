use pyo3::prelude::*;

use super::http::handle;

use crate::callbacks::CallbackScheduler;
use crate::conversion::{worker_http1_config_from_py, worker_http2_config_from_py};
use crate::tcp::ListenerSpec;
use crate::workers::{WorkerConfig, WorkerSignalSync};

#[pyclass(frozen, module = "granian._granian")]
pub struct WSGIWorker {
    config: WorkerConfig,
}

impl WSGIWorker {
    fn _serve_mtr(
        &self,
        py: Python,
        callback: Py<CallbackScheduler>,
        event_loop: &Bound<PyAny>,
        signal: Py<WorkerSignalSync>,
    ) {
        _ = pyo3_log::try_init();

        let worker_id = self.config.id;
        log::info!("Started worker-{worker_id}");

        let tcp_listener = self.config.tcp_listener();
        let http_mode = self.config.http_mode.clone();
        let http_upgrades = self.config.websockets_enabled;
        let http1_opts = self.config.http1_opts.clone();
        let http2_opts = self.config.http2_opts.clone();
        let backpressure = self.config.backpressure;
        let callback_wrapper = std::sync::Arc::new(callback);
        let rtpyloop = std::sync::Arc::new(event_loop.clone().unbind());

        let rt = py.allow_threads(|| {
            crate::runtime::init_runtime_mt(
                self.config.threads,
                self.config.blocking_threads,
                self.config.py_threads,
                self.config.py_threads_idle_timeout,
                rtpyloop,
            )
        });
        let rth = rt.handler();
        let tasks = tokio_util::task::TaskTracker::new();

        let (stx, mut srx) = tokio::sync::watch::channel(false);
        let main_loop = rt.inner.spawn(async move {
            crate::workers::loop_match!(
                http_mode,
                http_upgrades,
                tcp_listener,
                srx,
                backpressure,
                rth,
                callback_wrapper,
                |task| tasks.spawn(task),
                hyper_util::rt::TokioExecutor::new,
                http1_opts,
                http2_opts,
                hyper_util::rt::TokioIo::new,
                handle
            );

            log::info!("Stopping worker-{worker_id}");

            tasks.close();
            tasks.wait().await;

            Python::with_gil(|_| drop(callback_wrapper));
        });

        let pysig = signal.clone_ref(py);
        std::thread::spawn(move || {
            let pyrx = pysig.get().rx.lock().unwrap().take().unwrap();
            _ = pyrx.recv();
            stx.send(true).unwrap();

            while !main_loop.is_finished() {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }

            Python::with_gil(|py| {
                _ = pysig.get().release(py);
                drop(pysig);
            });
        });

        _ = signal.get().qs.call_method0(py, pyo3::intern!(py, "wait"));
    }

    fn _serve_str(
        &self,
        py: Python,
        callback: Py<CallbackScheduler>,
        event_loop: &Bound<PyAny>,
        signal: Py<WorkerSignalSync>,
    ) {
        _ = pyo3_log::try_init();

        let worker_id = self.config.id;
        log::info!("Started worker-{worker_id}");

        let (stx, srx) = tokio::sync::watch::channel(false);
        let mut workers = vec![];
        crate::workers::serve_str_inner!(self, handle, callback, event_loop, worker_id, workers, srx);

        let pysig = signal.clone_ref(py);
        std::thread::spawn(move || {
            let pyrx = pysig.get().rx.lock().unwrap().take().unwrap();
            _ = pyrx.recv();
            stx.send(true).unwrap();
            log::info!("Stopping worker-{worker_id}");
            while let Some(worker) = workers.pop() {
                worker.join().unwrap();
            }

            Python::with_gil(|py| {
                _ = pysig.get().release(py);
                drop(pysig);
            });
        });

        _ = signal.get().qs.call_method0(py, pyo3::intern!(py, "wait"));
    }

    fn _serve_mtr_ssl(
        &self,
        py: Python,
        callback: Py<CallbackScheduler>,
        event_loop: &Bound<PyAny>,
        signal: Py<WorkerSignalSync>,
    ) {
        _ = pyo3_log::try_init();

        let worker_id = self.config.id;
        log::info!("Started worker-{worker_id}");

        let tcp_listener = self.config.tcp_listener();
        let http_mode = self.config.http_mode.clone();
        let http_upgrades = self.config.websockets_enabled;
        let http1_opts = self.config.http1_opts.clone();
        let http2_opts = self.config.http2_opts.clone();
        let backpressure = self.config.backpressure;
        let tls_cfg = self.config.tls_cfg();
        let callback_wrapper = std::sync::Arc::new(callback);
        let rtpyloop = std::sync::Arc::new(event_loop.clone().unbind());

        let rt = py.allow_threads(|| {
            crate::runtime::init_runtime_mt(
                self.config.threads,
                self.config.blocking_threads,
                self.config.py_threads,
                self.config.py_threads_idle_timeout,
                rtpyloop,
            )
        });
        let rth = rt.handler();
        let tasks = tokio_util::task::TaskTracker::new();

        let (stx, mut srx) = tokio::sync::watch::channel(false);
        rt.inner.spawn(async move {
            crate::workers::loop_match_tls!(
                http_mode,
                http_upgrades,
                tcp_listener,
                tls_cfg,
                srx,
                backpressure,
                rth,
                callback_wrapper,
                |task| tasks.spawn(task),
                hyper_util::rt::TokioExecutor::new,
                http1_opts,
                http2_opts,
                hyper_util::rt::TokioIo::new,
                handle
            );

            log::info!("Stopping worker-{worker_id}");

            tasks.close();
            tasks.wait().await;

            Python::with_gil(|_| drop(callback_wrapper));
        });

        let pysig = signal.clone_ref(py);
        std::thread::spawn(move || {
            let pyrx = pysig.get().rx.lock().unwrap().take().unwrap();
            _ = pyrx.recv();
            stx.send(true).unwrap();

            Python::with_gil(|py| {
                _ = pysig.get().release(py);
                drop(pysig);
            });
        });

        _ = signal.get().qs.call_method0(py, pyo3::intern!(py, "wait"));
    }

    fn _serve_str_ssl(
        &self,
        py: Python,
        callback: Py<CallbackScheduler>,
        event_loop: &Bound<PyAny>,
        signal: Py<WorkerSignalSync>,
    ) {
        _ = pyo3_log::try_init();

        let worker_id = self.config.id;
        log::info!("Started worker-{worker_id}");

        let (stx, srx) = tokio::sync::watch::channel(false);
        let mut workers = vec![];
        crate::workers::serve_str_ssl_inner!(self, handle, callback, event_loop, worker_id, workers, srx);

        let pysig = signal.clone_ref(py);
        std::thread::spawn(move || {
            let pyrx = pysig.get().rx.lock().unwrap().take().unwrap();
            _ = pyrx.recv();
            stx.send(true).unwrap();
            log::info!("Stopping worker-{worker_id}");
            while let Some(worker) = workers.pop() {
                worker.join().unwrap();
            }

            Python::with_gil(|py| {
                _ = pysig.get().release(py);
                drop(pysig);
            });
        });

        _ = signal.get().qs.call_method0(py, pyo3::intern!(py, "wait"));
    }
}

#[pymethods]
impl WSGIWorker {
    #[new]
    #[pyo3(
        signature = (
            worker_id,
            sock,
            threads=1,
            blocking_threads=512,
            py_threads=1,
            py_threads_idle_timeout=30,
            backpressure=128,
            http_mode="1",
            http1_opts=None,
            http2_opts=None,
            ssl_enabled=false,
            ssl_cert=None,
            ssl_key=None,
            ssl_key_password=None
        )
    )]
    fn new(
        py: Python,
        worker_id: i32,
        sock: (Py<ListenerSpec>, Option<i32>),
        threads: usize,
        blocking_threads: usize,
        py_threads: usize,
        py_threads_idle_timeout: u64,
        backpressure: usize,
        http_mode: &str,
        http1_opts: Option<PyObject>,
        http2_opts: Option<PyObject>,
        ssl_enabled: bool,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
        ssl_key_password: Option<&str>,
    ) -> PyResult<Self> {
        Ok(Self {
            config: WorkerConfig::new(
                worker_id,
                sock,
                threads,
                blocking_threads,
                py_threads,
                py_threads_idle_timeout,
                backpressure,
                http_mode,
                worker_http1_config_from_py(py, http1_opts)?,
                worker_http2_config_from_py(py, http2_opts)?,
                false,
                ssl_enabled,
                ssl_cert,
                ssl_key,
                ssl_key_password,
            ),
        })
    }

    fn serve_mtr(
        &self,
        py: Python,
        callback: Py<CallbackScheduler>,
        event_loop: &Bound<PyAny>,
        signal: Py<WorkerSignalSync>,
    ) {
        match self.config.ssl_enabled {
            false => self._serve_mtr(py, callback, event_loop, signal),
            true => self._serve_mtr_ssl(py, callback, event_loop, signal),
        }
    }

    fn serve_str(
        &self,
        py: Python,
        callback: Py<CallbackScheduler>,
        event_loop: &Bound<PyAny>,
        signal: Py<WorkerSignalSync>,
    ) {
        match self.config.ssl_enabled {
            false => self._serve_str(py, callback, event_loop, signal),
            true => self._serve_str_ssl(py, callback, event_loop, signal),
        }
    }
}
