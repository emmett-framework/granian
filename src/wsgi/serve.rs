use futures::FutureExt;
use pyo3::prelude::*;

use super::http::handle;

use crate::callbacks::CallbackScheduler;
use crate::conversion::{worker_http1_config_from_py, worker_http2_config_from_py};
use crate::tcp::SocketHolder;
use crate::workers::{WorkerConfig, WorkerSignalSync};

#[pyclass(frozen, module = "granian._granian")]
pub struct WSGIWorker {
    config: WorkerConfig,
}

macro_rules! serve_mtr {
    ($func_name:ident, $http_mode:tt, $conn_method:ident, $ctx:ty, $svc:ident) => {
        fn $func_name(
            &self,
            py: Python,
            callback: Py<crate::callbacks::CallbackScheduler>,
            event_loop: &Bound<PyAny>,
            signal: Py<WorkerSignalSync>,
        ) {
            _ = pyo3_log::try_init();

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let tcp_listener = self.config.tcp_listener();
            #[allow(unused_variables)]
            let http1_opts = self.config.http1_opts.clone();
            #[allow(unused_variables)]
            let http2_opts = self.config.http2_opts.clone();
            let backpressure = self.config.backpressure.clone();

            let ctxw: Box<dyn crate::workers::WorkerCTX<CTX=$ctx>> = Box::new(crate::workers::Worker::new(<$ctx>::new(callback, self.config.static_files.clone())));
            let ctx = ctxw.get_ctx();

            let rtpyloop = std::sync::Arc::new(event_loop.clone().unbind());
            let rt = py.allow_threads(|| crate::runtime::init_runtime_mt(
                self.config.threads,
                self.config.blocking_threads,
                self.config.py_threads,
                self.config.py_threads_idle_timeout,
                rtpyloop,
            ));
            let rth = rt.handler();
            let tasks = tokio_util::task::TaskTracker::new();
            let (stx, mut srx) = tokio::sync::watch::channel(false);

            let main_loop = rt.inner.spawn(async move {
                crate::workers::gen_accept!(
                    plain
                    $http_mode
                    $conn_method,
                    ctx,
                    handle,
                    $svc,
                    tcp_listener,
                    srx,
                    backpressure,
                    rth,
                    |task| tasks.spawn(task),
                    hyper_util::rt::TokioExecutor::new,
                    http1_opts,
                    http2_opts,
                    hyper_util::rt::TokioIo::new
                );

                log::info!("Stopping worker-{}", worker_id);

                tasks.close();
                tasks.wait().await;

                Python::with_gil(|_| drop(ctx));
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
    };
}

macro_rules! serve_mtr_ssl {
    ($func_name:ident, $http_mode:tt, $conn_method:ident, $ctx:ty, $svc:ident) => {
        fn $func_name(
            &self,
            py: Python,
            callback: Py<crate::callbacks::CallbackScheduler>,
            event_loop: &Bound<PyAny>,
            signal: Py<WorkerSignalSync>,
        ) {
            _ = pyo3_log::try_init();

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let tcp_listener = self.config.tcp_listener();
            #[allow(unused_variables)]
            let http1_opts = self.config.http1_opts.clone();
            #[allow(unused_variables)]
            let http2_opts = self.config.http2_opts.clone();
            let backpressure = self.config.backpressure.clone();
            let tls_cfg = self.config.tls_cfg();

            let ctxw: Box<dyn crate::workers::WorkerCTX<CTX=$ctx>> = Box::new(crate::workers::Worker::new(<$ctx>::new(callback, self.config.static_files.clone())));
            let ctx = ctxw.get_ctx();

            let rtpyloop = std::sync::Arc::new(event_loop.clone().unbind());
            let rt = py.allow_threads(|| crate::runtime::init_runtime_mt(
                self.config.threads,
                self.config.blocking_threads,
                self.config.py_threads,
                self.config.py_threads_idle_timeout,
                rtpyloop,
            ));
            let rth = rt.handler();
            let tasks = tokio_util::task::TaskTracker::new();
            let (stx, mut srx) = tokio::sync::watch::channel(false);

            let main_loop = rt.inner.spawn(async move {
                crate::workers::gen_accept!(
                    tls
                    $http_mode
                    $conn_method,
                    tls_cfg,
                    ctx,
                    handle,
                    $svc,
                    tcp_listener,
                    srx,
                    backpressure,
                    rth,
                    |task| tasks.spawn(task),
                    hyper_util::rt::TokioExecutor::new,
                    http1_opts,
                    http2_opts,
                    hyper_util::rt::TokioIo::new
                );

                log::info!("Stopping worker-{}", worker_id);

                tasks.close();
                tasks.wait().await;

                Python::with_gil(|_| drop(ctx));
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
    };
}

macro_rules! serve_str {
    ($func_name:ident, $http_mode:tt, $conn_method:ident, $ctx:ty, $svc:ident) => {
        fn $func_name(
            &self,
            py: Python,
            callback: Py<crate::callbacks::CallbackScheduler>,
            event_loop: &Bound<PyAny>,
            signal: Py<WorkerSignalSync>,
        ) {
            _ = pyo3_log::try_init();

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let (stx, srx) = tokio::sync::watch::channel(false);
            let mut workers = vec![];
            crate::workers::serve_str_inner!(
                $http_mode,
                $conn_method,
                handle,
                $ctx,
                $svc,
                self,
                callback,
                event_loop,
                worker_id,
                workers,
                srx
            );

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
    };
}

macro_rules! serve_str_ssl {
    ($func_name:ident, $http_mode:tt, $conn_method:ident, $ctx:ty, $svc:ident) => {
        fn $func_name(
            &self,
            py: Python,
            callback: Py<crate::callbacks::CallbackScheduler>,
            event_loop: &Bound<PyAny>,
            signal: Py<WorkerSignalSync>,
        ) {
            _ = pyo3_log::try_init();

            let worker_id = self.config.id;
            log::info!("Started worker-{}", worker_id);

            let (stx, srx) = tokio::sync::watch::channel(false);
            let mut workers = vec![];
            crate::workers::serve_str_ssl_inner!(
                $http_mode,
                $conn_method,
                handle,
                $ctx,
                $svc,
                self,
                callback,
                event_loop,
                worker_id,
                workers,
                srx
            );

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
    };
}

impl WSGIWorker {
    serve_mtr!(
        _serve_mtr_http_plain_auto_base,
        auto,
        serve_connection,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_mtr!(
        _serve_mtr_http_plain_auto_file,
        auto,
        serve_connection,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_mtr!(
        _serve_mtr_http_plain_autou_base,
        auto,
        serve_connection_with_upgrades,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_mtr!(
        _serve_mtr_http_plain_autou_file,
        auto,
        serve_connection_with_upgrades,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_mtr!(
        _serve_mtr_http_plain_1_base,
        1,
        connection_builder_h1,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_mtr!(
        _serve_mtr_http_plain_1_file,
        1,
        connection_builder_h1,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_mtr!(
        _serve_mtr_http_plain_1u_base,
        1,
        connection_builder_h1u,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_mtr!(
        _serve_mtr_http_plain_1u_file,
        1,
        connection_builder_h1u,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_mtr!(
        _serve_mtr_http_plain_2_base,
        2,
        serve_connection,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_mtr!(
        _serve_mtr_http_plain_2_file,
        2,
        serve_connection,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_mtr_ssl!(
        _serve_mtr_http_tls_auto_base,
        auto,
        serve_connection,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_mtr_ssl!(
        _serve_mtr_http_tls_auto_file,
        auto,
        serve_connection,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_mtr_ssl!(
        _serve_mtr_http_tls_autou_base,
        auto,
        serve_connection_with_upgrades,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_mtr_ssl!(
        _serve_mtr_http_tls_autou_file,
        auto,
        serve_connection_with_upgrades,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_mtr_ssl!(
        _serve_mtr_http_tls_1_base,
        1,
        connection_builder_h1,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_mtr_ssl!(
        _serve_mtr_http_tls_1_file,
        1,
        connection_builder_h1,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_mtr_ssl!(
        _serve_mtr_http_tls_1u_base,
        1,
        connection_builder_h1u,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_mtr_ssl!(
        _serve_mtr_http_tls_1u_file,
        1,
        connection_builder_h1u,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_mtr_ssl!(
        _serve_mtr_http_tls_2_base,
        2,
        serve_connection,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_mtr_ssl!(
        _serve_mtr_http_tls_2_file,
        2,
        serve_connection,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_str!(
        _serve_str_http_plain_auto_base,
        auto,
        serve_connection,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_str!(
        _serve_str_http_plain_auto_file,
        auto,
        serve_connection,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_str!(
        _serve_str_http_plain_autou_base,
        auto,
        serve_connection_with_upgrades,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_str!(
        _serve_str_http_plain_autou_file,
        auto,
        serve_connection_with_upgrades,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_str!(
        _serve_str_http_plain_1_base,
        1,
        connection_builder_h1,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_str!(
        _serve_str_http_plain_1_file,
        1,
        connection_builder_h1,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_str!(
        _serve_str_http_plain_1u_base,
        1,
        connection_builder_h1u,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_str!(
        _serve_str_http_plain_1u_file,
        1,
        connection_builder_h1u,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_str!(
        _serve_str_http_plain_2_base,
        2,
        serve_connection,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_str!(
        _serve_str_http_plain_2_file,
        2,
        serve_connection,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_str_ssl!(
        _serve_str_http_tls_auto_base,
        auto,
        serve_connection,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_str_ssl!(
        _serve_str_http_tls_auto_file,
        auto,
        serve_connection,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_str_ssl!(
        _serve_str_http_tls_autou_base,
        auto,
        serve_connection_with_upgrades,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_str_ssl!(
        _serve_str_http_tls_autou_file,
        auto,
        serve_connection_with_upgrades,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_str_ssl!(
        _serve_str_http_tls_1_base,
        1,
        connection_builder_h1,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_str_ssl!(
        _serve_str_http_tls_1_file,
        1,
        connection_builder_h1,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_str_ssl!(
        _serve_str_http_tls_1u_base,
        1,
        connection_builder_h1u,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_str_ssl!(
        _serve_str_http_tls_1u_file,
        1,
        connection_builder_h1u,
        crate::workers::WorkerCTXFiles,
        service_files
    );
    serve_str_ssl!(
        _serve_str_http_tls_2_base,
        2,
        serve_connection,
        crate::workers::WorkerCTXBase,
        service_app
    );
    serve_str_ssl!(
        _serve_str_http_tls_2_file,
        2,
        serve_connection,
        crate::workers::WorkerCTXFiles,
        service_files
    );
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
            static_files=None,
            ssl_enabled=false,
            ssl_cert=None,
            ssl_key=None,
            ssl_key_password=None,
            ssl_ca=None,
            ssl_crl=vec![],
            ssl_client_verify=false,
        )
    )]
    fn new(
        py: Python,
        worker_id: i32,
        sock: Py<SocketHolder>,
        threads: usize,
        blocking_threads: usize,
        py_threads: usize,
        py_threads_idle_timeout: u64,
        backpressure: usize,
        http_mode: &str,
        http1_opts: Option<PyObject>,
        http2_opts: Option<PyObject>,
        static_files: Option<(String, String, String)>,
        ssl_enabled: bool,
        ssl_cert: Option<String>,
        ssl_key: Option<String>,
        ssl_key_password: Option<String>,
        ssl_ca: Option<String>,
        ssl_crl: Vec<String>,
        ssl_client_verify: bool,
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
                static_files,
                ssl_enabled,
                ssl_cert,
                ssl_key,
                ssl_key_password,
                ssl_ca,
                ssl_crl,
                ssl_client_verify,
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
        match (
            &self.config.http_mode[..],
            self.config.tls_opts.is_some(),
            self.config.static_files.is_some(),
        ) {
            ("auto", false, false) => self._serve_mtr_http_plain_auto_base(py, callback, event_loop, signal),
            ("auto", false, true) => self._serve_mtr_http_plain_auto_file(py, callback, event_loop, signal),
            ("auto", true, false) => self._serve_mtr_http_tls_auto_base(py, callback, event_loop, signal),
            ("auto", true, true) => self._serve_mtr_http_tls_auto_file(py, callback, event_loop, signal),
            ("1", false, false) => self._serve_mtr_http_plain_1_base(py, callback, event_loop, signal),
            ("1", false, true) => self._serve_mtr_http_plain_1_file(py, callback, event_loop, signal),
            ("1", true, false) => self._serve_mtr_http_tls_1_base(py, callback, event_loop, signal),
            ("1", true, true) => self._serve_mtr_http_tls_1_file(py, callback, event_loop, signal),
            ("2", false, false) => self._serve_mtr_http_plain_2_base(py, callback, event_loop, signal),
            ("2", false, true) => self._serve_mtr_http_plain_2_file(py, callback, event_loop, signal),
            ("2", true, false) => self._serve_mtr_http_tls_2_base(py, callback, event_loop, signal),
            ("2", true, true) => self._serve_mtr_http_tls_2_file(py, callback, event_loop, signal),
            _ => unreachable!(),
        }
    }

    fn serve_str(
        &self,
        py: Python,
        callback: Py<CallbackScheduler>,
        event_loop: &Bound<PyAny>,
        signal: Py<WorkerSignalSync>,
    ) {
        match (
            &self.config.http_mode[..],
            self.config.tls_opts.is_some(),
            self.config.static_files.is_some(),
        ) {
            ("auto", false, false) => self._serve_str_http_plain_auto_base(py, callback, event_loop, signal),
            ("auto", false, true) => self._serve_str_http_plain_auto_file(py, callback, event_loop, signal),
            ("auto", true, false) => self._serve_str_http_tls_auto_base(py, callback, event_loop, signal),
            ("auto", true, true) => self._serve_str_http_tls_auto_file(py, callback, event_loop, signal),
            ("1", false, false) => self._serve_str_http_plain_1_base(py, callback, event_loop, signal),
            ("1", false, true) => self._serve_str_http_plain_1_file(py, callback, event_loop, signal),
            ("1", true, false) => self._serve_str_http_tls_1_base(py, callback, event_loop, signal),
            ("1", true, true) => self._serve_str_http_tls_1_file(py, callback, event_loop, signal),
            ("2", false, false) => self._serve_str_http_plain_2_base(py, callback, event_loop, signal),
            ("2", false, true) => self._serve_str_http_plain_2_file(py, callback, event_loop, signal),
            ("2", true, false) => self._serve_str_http_tls_2_base(py, callback, event_loop, signal),
            ("2", true, true) => self._serve_str_http_tls_2_file(py, callback, event_loop, signal),
            _ => unreachable!(),
        }
    }
}
