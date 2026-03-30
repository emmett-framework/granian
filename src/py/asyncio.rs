use pyo3::{IntoPyObjectExt, exceptions::PyStopIteration, prelude::*};
use std::sync::{Arc, OnceLock, RwLock, atomic};
use tokio::sync::Notify;

use crate::runtime::{ContextExt, Runtime};

#[pyclass(frozen, freelist = 128, module = "granian._granian")]
pub(crate) struct PyEmptyAwaitable;

#[pymethods]
impl PyEmptyAwaitable {
    fn __await__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(&self) -> Option<()> {
        None
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct PyDoneAwaitable {
    result: PyResult<Py<PyAny>>,
}

impl PyDoneAwaitable {
    pub(crate) fn new(result: PyResult<Py<PyAny>>) -> Self {
        Self { result }
    }
}

#[pymethods]
impl PyDoneAwaitable {
    fn __await__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(&self, py: Python) -> PyResult<Py<PyAny>> {
        self.result
            .as_ref()
            .map_err(|v| v.clone_ref(py))
            .map(|v| Err(PyStopIteration::new_err(v.clone_ref(py))))?
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct PyErrAwaitable {
    result: PyResult<()>,
}

impl PyErrAwaitable {
    pub(crate) fn new(result: PyResult<()>) -> Self {
        Self { result }
    }
}

#[pymethods]
impl PyErrAwaitable {
    fn __await__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(&self, py: Python) -> PyResult<()> {
        Err(self.result.as_ref().err().unwrap().clone_ref(py))
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct PyIterAwaitable {
    result: OnceLock<PyResult<Py<PyAny>>>,
}

impl PyIterAwaitable {
    pub(crate) fn new() -> Self {
        Self {
            result: OnceLock::new(),
        }
    }

    #[inline]
    pub(crate) fn set_result(pyself: Py<Self>, py: Python, result: FutureResultToPy) {
        _ = pyself.get().result.set(result.into_pyobject(py).map(Bound::unbind));
        pyself.drop_ref(py);
    }
}

#[pymethods]
impl PyIterAwaitable {
    fn __await__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(&self, py: Python) -> PyResult<Option<Py<PyAny>>> {
        if let Some(res) = self.result.get() {
            return res
                .as_ref()
                .map_err(|err| err.clone_ref(py))
                .map(|v| Err(PyStopIteration::new_err(v.clone_ref(py))))?;
        }

        Ok(Some(py.None()))
    }
}

#[repr(u8)]
enum PyFutureAwaitableState {
    Pending = 0,
    Completed = 1,
    Cancelled = 2,
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct PyFutureAwaitable {
    state: atomic::AtomicU8,
    result: OnceLock<PyResult<Py<PyAny>>>,
    event_loop: Py<PyAny>,
    cancel_tx: Arc<Notify>,
    cancel_msg: OnceLock<Py<PyAny>>,
    py_block: atomic::AtomicBool,
    ack: RwLock<Option<(Py<PyAny>, Py<pyo3::types::PyDict>)>>,
}

impl PyFutureAwaitable {
    pub(crate) fn new(event_loop: Py<PyAny>) -> Self {
        Self {
            state: atomic::AtomicU8::new(PyFutureAwaitableState::Pending as u8),
            result: OnceLock::new(),
            event_loop,
            cancel_tx: Arc::new(Notify::new()),
            cancel_msg: OnceLock::new(),
            py_block: true.into(),
            ack: RwLock::new(None),
        }
    }

    pub fn to_spawn(self, py: Python) -> PyResult<(Py<PyFutureAwaitable>, Arc<Notify>)> {
        let cancel_tx = self.cancel_tx.clone();
        Ok((Py::new(py, self)?, cancel_tx))
    }

    pub(crate) fn set_result(pyself: Py<Self>, py: Python, result: FutureResultToPy) {
        let rself = pyself.get();

        _ = rself.result.set(result.into_pyobject(py).map(Bound::unbind));
        if rself
            .state
            .compare_exchange(
                PyFutureAwaitableState::Pending as u8,
                PyFutureAwaitableState::Completed as u8,
                atomic::Ordering::Release,
                atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            pyself.drop_ref(py);
            return;
        }

        if let Some((cb, ctx)) = {
            let ack = rself.ack.read().unwrap();
            ack.as_ref().map(|(cb, ctx)| (cb.clone_ref(py), ctx.clone_ref(py)))
        } {
            _ = rself.event_loop.clone_ref(py).call_method(
                py,
                pyo3::intern!(py, "call_soon_threadsafe"),
                (cb, pyself.clone_ref(py)),
                Some(ctx.bind(py)),
            );
        }
        pyself.drop_ref(py);
    }
}

#[pymethods]
impl PyFutureAwaitable {
    fn __await__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(pyself: PyRef<'_, Self>) -> PyResult<Option<PyRef<'_, Self>>> {
        if pyself.state.load(atomic::Ordering::Acquire) == PyFutureAwaitableState::Completed as u8 {
            let py = pyself.py();
            return pyself
                .result
                .get()
                .unwrap()
                .as_ref()
                .map_err(|err| err.clone_ref(py))
                .map(|v| Err(PyStopIteration::new_err(v.clone_ref(py))))?;
        }

        Ok(Some(pyself))
    }

    #[getter(_asyncio_future_blocking)]
    fn get_block(&self) -> bool {
        self.py_block.load(atomic::Ordering::Relaxed)
    }

    #[setter(_asyncio_future_blocking)]
    fn set_block(&self, val: bool) {
        self.py_block.store(val, atomic::Ordering::Relaxed);
    }

    fn get_loop(&self, py: Python) -> Py<PyAny> {
        self.event_loop.clone_ref(py)
    }

    #[pyo3(signature = (cb, context=None))]
    fn add_done_callback(pyself: PyRef<'_, Self>, cb: Py<PyAny>, context: Option<Py<PyAny>>) -> PyResult<()> {
        let py = pyself.py();
        let kwctx = pyo3::types::PyDict::new(py);
        kwctx.set_item(pyo3::intern!(py, "context"), context)?;

        {
            let mut ack = pyself.ack.write().unwrap();
            if pyself.state.load(atomic::Ordering::Acquire) == PyFutureAwaitableState::Pending as u8 {
                *ack = Some((cb, kwctx.unbind()));
                return Ok(());
            }
        }

        let event_loop = pyself.event_loop.clone_ref(py);
        event_loop.call_method(py, pyo3::intern!(py, "call_soon"), (cb, pyself), Some(&kwctx))?;

        Ok(())
    }

    #[allow(unused)]
    fn remove_done_callback(&self, cb: Py<PyAny>) -> i32 {
        let mut ack = self.ack.write().unwrap();
        *ack = None;
        1
    }

    #[allow(unused)]
    #[pyo3(signature = (msg=None))]
    fn cancel(pyself: PyRef<'_, Self>, msg: Option<Py<PyAny>>) -> bool {
        if pyself
            .state
            .compare_exchange(
                PyFutureAwaitableState::Pending as u8,
                PyFutureAwaitableState::Cancelled as u8,
                atomic::Ordering::Release,
                atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            return false;
        }

        if let Some(cancel_msg) = msg {
            _ = pyself.cancel_msg.set(cancel_msg);
        }
        pyself.cancel_tx.notify_one();

        let ack = pyself.ack.read().unwrap();
        if let Some((cb, ctx)) = &*ack {
            let py = pyself.py();
            let event_loop = pyself.event_loop.clone_ref(py);
            let cb = cb.clone_ref(py);
            let ctx = ctx.clone_ref(py);
            drop(ack);

            let _ = event_loop.call_method(py, pyo3::intern!(py, "call_soon"), (cb, pyself), Some(ctx.bind(py)));
        }

        true
    }

    fn done(&self) -> bool {
        self.state.load(atomic::Ordering::Acquire) != PyFutureAwaitableState::Pending as u8
    }

    fn cancelled(&self) -> bool {
        self.state.load(atomic::Ordering::Acquire) == PyFutureAwaitableState::Cancelled as u8
    }

    fn result(&self, py: Python) -> PyResult<Py<PyAny>> {
        let state = self.state.load(atomic::Ordering::Acquire);

        if state == PyFutureAwaitableState::Completed as u8 {
            return self
                .result
                .get()
                .unwrap()
                .as_ref()
                .map(|v| v.clone_ref(py))
                .map_err(|err| err.clone_ref(py));
        }
        if state == PyFutureAwaitableState::Cancelled as u8 {
            let msg = self
                .cancel_msg
                .get()
                .unwrap_or(&"Future cancelled.".into_py_any(py).unwrap())
                .clone_ref(py);
            return Err(pyo3::exceptions::asyncio::CancelledError::new_err(msg));
        }
        Err(pyo3::exceptions::asyncio::InvalidStateError::new_err(
            "Result is not ready.",
        ))
    }

    fn exception(&self, py: Python) -> PyResult<Py<PyAny>> {
        let state = self.state.load(atomic::Ordering::Acquire);

        if state == PyFutureAwaitableState::Completed as u8 {
            return self
                .result
                .get()
                .unwrap()
                .as_ref()
                .map_or_else(|err| err.clone_ref(py).into_py_any(py), |_| Ok(py.None()));
        }
        if state == PyFutureAwaitableState::Cancelled as u8 {
            let msg = self
                .cancel_msg
                .get()
                .unwrap_or(&"Future cancelled.".into_py_any(py).unwrap())
                .clone_ref(py);
            return Err(pyo3::exceptions::asyncio::CancelledError::new_err(msg));
        }
        Err(pyo3::exceptions::asyncio::InvalidStateError::new_err(
            "Exception is not set.",
        ))
    }
}

#[pyclass(frozen)]
pub(crate) struct PyFutureDoneCallback {
    pub cancel_tx: Arc<Notify>,
}

#[pymethods]
impl PyFutureDoneCallback {
    pub fn __call__(&self, fut: Bound<PyAny>) -> PyResult<()> {
        let py = fut.py();

        if { fut.getattr(pyo3::intern!(py, "cancelled"))?.call0()?.is_truthy() }.unwrap_or(false) {
            self.cancel_tx.notify_one();
        }

        Ok(())
    }
}

#[pyclass(frozen)]
pub(crate) struct PyFutureResultSetter;

#[pymethods]
impl PyFutureResultSetter {
    pub fn __call__(&self, target: Bound<PyAny>, value: Bound<PyAny>) {
        let _ = target.call1((value,));
    }
}

pub(crate) enum FutureResultToPy {
    None,
    Err(PyResult<()>),
    Bytes(hyper::body::Bytes),
    BytesEof((hyper::body::Bytes, bool)),
    ASGIMessage(crate::asgi::types::ASGIMessageType),
    ASGIWSMessage(tokio_tungstenite::tungstenite::Message),
    RSGIWSAccept((crate::rsgi::io::WebsocketReader, crate::rsgi::io::WebsocketWriter)),
    RSGIWSMessage(tokio_tungstenite::tungstenite::Message),
}

impl<'p> IntoPyObject<'p> for FutureResultToPy {
    type Target = PyAny;
    type Output = Bound<'p, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'p>) -> Result<Self::Output, Self::Error> {
        match self {
            Self::None => Ok(py.None().into_bound(py)),
            Self::Err(res) => Err(res.err().unwrap()),
            Self::Bytes(inner) => inner.into_bound_py_any(py),
            Self::BytesEof(inner) => inner.into_bound_py_any(py),
            Self::ASGIMessage(message) => crate::asgi::conversion::message_into_py(py, message),
            Self::ASGIWSMessage(message) => crate::asgi::conversion::ws_message_into_py(py, message),
            Self::RSGIWSAccept(obj) => obj.into_bound_py_any(py),
            Self::RSGIWSMessage(message) => crate::rsgi::conversion::ws_message_into_py(py, message),
        }
    }
}

#[inline(always)]
pub(crate) fn empty_future_into_asyncio(py: Python) -> PyResult<Bound<PyAny>> {
    PyEmptyAwaitable.into_bound_py_any(py)
}

#[inline(always)]
pub(crate) fn done_future_into_asyncio(py: Python, result: PyResult<Py<PyAny>>) -> PyResult<Bound<PyAny>> {
    PyDoneAwaitable::new(result).into_bound_py_any(py)
}

#[inline(always)]
pub(crate) fn err_future_into_asyncio(py: Python, err: PyResult<()>) -> PyResult<Bound<PyAny>> {
    PyErrAwaitable::new(err).into_bound_py_any(py)
}

// NOTE:
//  `future_into_py_iter` relies on what CPython refers as "bare yield".
//  This is generally ~55% faster than `pyo3_asyncio.future_into_py` implementation.
//  It consumes more cpu-cycles than `future_into_py_futlike`,
//  but for "quick" operations it's something like 12% faster.
#[allow(dead_code, unused_must_use)]
pub(crate) fn future_into_asyncio_iter<R, F>(rt: R, py: Python, fut: F) -> PyResult<Bound<PyAny>>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = FutureResultToPy> + Send + 'static,
{
    let aw = Py::new(py, PyIterAwaitable::new())?;
    let py_fut = aw.clone_ref(py);
    let rth = rt.clone();

    rt.spawn(async move {
        let result = fut.await;
        rth.spawn_blocking(move |py| PyIterAwaitable::set_result(aw, py, result));
    });

    Ok(py_fut.into_any().into_bound(py))
}

// NOTE:
//  `future_into_py_futlike` relies on an `asyncio.Future` like implementation.
//  This is generally ~38% faster than `pyo3_asyncio.future_into_py` implementation.
//  It won't consume more cpu-cycles than standard asyncio implementation,
//  and for "long" operations it's something like 6% faster than `future_into_py_iter`.
#[allow(unused_must_use)]
#[cfg(unix)]
pub(crate) fn future_into_asyncio_futlike<R, F>(rt: R, py: Python, fut: F) -> PyResult<Bound<PyAny>>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = FutureResultToPy> + Send + 'static,
{
    let event_loop = rt.py_event_loop(py);
    let (aw, cancel_tx) = PyFutureAwaitable::new(event_loop).to_spawn(py)?;
    let py_fut = aw.clone_ref(py);
    let rth = rt.clone();

    rt.spawn(async move {
        tokio::select! {
            biased;
            result = fut => rth.spawn_blocking(move |py| PyFutureAwaitable::set_result(aw, py, result)),
            () = cancel_tx.notified() => rth.spawn_blocking(move |py| aw.drop_ref(py)),
        }
    });

    Ok(py_fut.into_any().into_bound(py))
}

#[allow(unused_must_use)]
#[cfg(windows)]
pub(crate) fn future_into_asyncio_futlike<R, F>(rt: R, py: Python, fut: F) -> PyResult<Bound<PyAny>>
where
    R: Runtime + ContextExt + Clone,
    F: Future<Output = FutureResultToPy> + Send + 'static,
{
    let event_loop = rt.py_event_loop(py);
    let event_loop_ref = event_loop.clone_ref(py);
    let cancel_tx = Arc::new(tokio::sync::Notify::new());
    let rth = rt.clone();

    let py_fut = event_loop.call_method0(py, pyo3::intern!(py, "create_future"))?;
    py_fut.call_method1(
        py,
        pyo3::intern!(py, "add_done_callback"),
        (PyFutureDoneCallback {
            cancel_tx: cancel_tx.clone(),
        },),
    )?;
    let fut_ref = py_fut.clone_ref(py);

    rt.spawn(async move {
        tokio::select! {
            biased;
            result = fut => {
                rth.spawn_blocking(move |py| {
                    let pyres = result.into_pyobject(py).map(Bound::unbind);
                    let (cb, value) = match pyres {
                        Ok(val) => (fut_ref.getattr(py, pyo3::intern!(py, "set_result")).unwrap(), val),
                        Err(err) => (fut_ref.getattr(py, pyo3::intern!(py, "set_exception")).unwrap(), err.into_py_any(py).unwrap())
                    };
                    let _ = event_loop_ref.call_method1(py, pyo3::intern!(py, "call_soon_threadsafe"), (PyFutureResultSetter, cb, value));
                    fut_ref.drop_ref(py);
                    event_loop_ref.drop_ref(py);
                });
            },
            () = cancel_tx.notified() => {
                rth.spawn_blocking(move |py| {
                    fut_ref.drop_ref(py);
                    event_loop_ref.drop_ref(py);
                });
            }
        }
    });

    Ok(py_fut.into_bound(py))
}
