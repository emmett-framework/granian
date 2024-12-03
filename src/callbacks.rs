use pyo3::{exceptions::PyStopIteration, prelude::*, IntoPyObjectExt};

use std::sync::{atomic, Arc, OnceLock, RwLock};
use tokio::sync::Notify;

pub(crate) type ArcCBScheduler = Arc<Py<CallbackScheduler>>;

#[pyclass(frozen, subclass, module = "granian._granian")]
pub(crate) struct CallbackScheduler {
    pub cb: PyObject,
    #[pyo3(get)]
    _loop: PyObject,
    #[pyo3(get)]
    _ctx: PyObject,
    schedule_fn: OnceLock<PyObject>,
    aio_tenter: PyObject,
    aio_texit: PyObject,
    pyname_aioblock: PyObject,
    pyname_aiosend: PyObject,
    pyname_aiothrow: PyObject,
    pyname_donecb: PyObject,
    pyname_loopcs: PyObject,
    pynone: PyObject,
    pyfalse: PyObject,
}

impl CallbackScheduler {
    #[inline]
    pub(crate) fn schedule(&self, _py: Python, watcher: &PyObject) {
        let cbarg = watcher.as_ptr();
        let sched = self.schedule_fn.get().unwrap().as_ptr();

        unsafe {
            pyo3::ffi::PyObject_CallOneArg(sched, cbarg);
        }
    }

    pub(crate) fn send(pyself: Py<Self>, py: Python, coro: PyObject) {
        let rself = pyself.get();
        let ptr = pyself.as_ptr();

        unsafe {
            pyo3::ffi::PyObject_CallOneArg(rself.aio_tenter.as_ptr(), ptr);
        }

        if let Ok(res) = unsafe {
            let res = pyo3::ffi::PyObject_CallMethodOneArg(
                coro.as_ptr(),
                rself.pyname_aiosend.as_ptr(),
                rself.pynone.as_ptr(),
            );
            Bound::from_owned_ptr_or_err(py, res)
        } {
            if unsafe {
                let vptr = pyo3::ffi::PyObject_GetAttr(res.as_ptr(), rself.pyname_aioblock.as_ptr());
                Bound::from_owned_ptr_or_err(py, vptr)
                    .map(|v| v.extract::<bool>().unwrap_or(false))
                    .unwrap_or(false)
            } {
                let waker = Py::new(
                    py,
                    CallbackSchedulerWaker {
                        sched: pyself.clone_ref(py),
                        coro,
                    },
                )
                .unwrap();
                let resp = res.as_ptr();

                unsafe {
                    pyo3::ffi::PyObject_SetAttr(resp, rself.pyname_aioblock.as_ptr(), rself.pyfalse.as_ptr());
                    pyo3::ffi::PyObject_CallMethodOneArg(resp, rself.pyname_donecb.as_ptr(), waker.as_ptr());
                }
            } else {
                let sref = Py::new(
                    py,
                    CallbackSchedulerRef {
                        sched: pyself.clone_ref(py),
                        coro,
                    },
                )
                .unwrap();

                unsafe {
                    pyo3::ffi::PyObject_CallMethodOneArg(
                        #[allow(clippy::used_underscore_binding)]
                        rself._loop.as_ptr(),
                        rself.pyname_loopcs.as_ptr(),
                        sref.as_ptr(),
                    );
                }
            }
        }

        unsafe {
            pyo3::ffi::PyObject_CallOneArg(rself.aio_texit.as_ptr(), ptr);
        }
    }

    #[inline]
    pub(crate) fn throw(pyself: Py<Self>, _py: Python, coro: PyObject, err: PyObject) {
        let rself = pyself.get();
        let ptr = pyself.as_ptr();

        unsafe {
            let corom = pyo3::ffi::PyObject_GetAttr(coro.as_ptr(), rself.pyname_aiothrow.as_ptr());
            pyo3::ffi::PyObject_CallOneArg(rself.aio_tenter.as_ptr(), ptr);
            pyo3::ffi::PyObject_CallOneArg(corom, err.as_ptr());
            pyo3::ffi::PyObject_CallOneArg(rself.aio_texit.as_ptr(), ptr);
        }
    }
}

#[pymethods]
impl CallbackScheduler {
    #[new]
    fn new(
        py: Python,
        event_loop: PyObject,
        ctx: PyObject,
        cb: PyObject,
        aio_tenter: PyObject,
        aio_texit: PyObject,
    ) -> Self {
        Self {
            _loop: event_loop,
            _ctx: ctx,
            schedule_fn: OnceLock::new(),
            cb,
            aio_tenter,
            aio_texit,
            pyfalse: false.into_py_any(py).unwrap(),
            pynone: py.None(),
            pyname_aioblock: pyo3::intern!(py, "_asyncio_future_blocking").into_py_any(py).unwrap(),
            pyname_aiosend: pyo3::intern!(py, "send").into_py_any(py).unwrap(),
            pyname_aiothrow: pyo3::intern!(py, "throw").into_py_any(py).unwrap(),
            pyname_donecb: pyo3::intern!(py, "add_done_callback").into_py_any(py).unwrap(),
            pyname_loopcs: pyo3::intern!(py, "call_soon").into_py_any(py).unwrap(),
        }
    }

    #[setter(_schedule_fn)]
    fn _set_schedule_fn(&self, val: PyObject) {
        self.schedule_fn.set(val).unwrap();
    }

    fn _run(pyself: Py<Self>, py: Python, coro: PyObject) {
        CallbackScheduler::send(pyself, py, coro);
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct CallbackSchedulerWaker {
    sched: Py<CallbackScheduler>,
    coro: PyObject,
}

#[pymethods]
impl CallbackSchedulerWaker {
    fn __call__(&self, py: Python, fut: PyObject) {
        match fut.call_method0(py, pyo3::intern!(py, "result")) {
            Ok(_) => CallbackScheduler::send(self.sched.clone_ref(py), py, self.coro.clone_ref(py)),
            Err(err) => CallbackScheduler::throw(
                self.sched.clone_ref(py),
                py,
                self.coro.clone_ref(py),
                err.into_py_any(py).unwrap(),
            ),
        }
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct CallbackSchedulerRef {
    sched: Py<CallbackScheduler>,
    coro: PyObject,
}

#[pymethods]
impl CallbackSchedulerRef {
    fn __call__(&self, py: Python) {
        CallbackScheduler::send(self.sched.clone_ref(py), py, self.coro.clone_ref(py));
    }
}

#[pyclass(frozen, module = "granian._granian")]
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
pub(crate) struct PyIterAwaitable {
    result: RwLock<Option<PyResult<PyObject>>>,
}

#[cfg(not(target_os = "linux"))]
impl PyIterAwaitable {
    pub(crate) fn new() -> Self {
        Self {
            result: RwLock::new(None),
        }
    }

    pub(crate) fn set_result(&self, result: PyResult<PyObject>) {
        let mut res = self.result.write().unwrap();
        *res = Some(result);
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

    fn __next__(&self, py: Python) -> PyResult<Option<PyObject>> {
        if let Ok(res) = self.result.try_read() {
            if let Some(ref res) = *res {
                return res
                    .as_ref()
                    .map_err(|err| err.clone_ref(py))
                    .map(|v| Err(PyStopIteration::new_err(v.clone_ref(py))))?;
            }
        };
        Ok(Some(py.None()))
    }
}

enum PyFutureAwaitableState {
    Pending,
    Completed(PyResult<PyObject>),
    Cancelled,
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct PyFutureAwaitable {
    state: RwLock<PyFutureAwaitableState>,
    event_loop: PyObject,
    cancel_tx: Arc<Notify>,
    py_block: atomic::AtomicBool,
    ack: RwLock<Option<(PyObject, Py<pyo3::types::PyDict>)>>,
}

impl PyFutureAwaitable {
    pub(crate) fn new(event_loop: PyObject) -> Self {
        Self {
            state: RwLock::new(PyFutureAwaitableState::Pending),
            event_loop,
            cancel_tx: Arc::new(Notify::new()),
            py_block: true.into(),
            ack: RwLock::new(None),
        }
    }

    pub fn to_spawn(self, py: Python) -> PyResult<(Py<PyFutureAwaitable>, Arc<Notify>)> {
        let cancel_tx = self.cancel_tx.clone();
        Ok((Py::new(py, self)?, cancel_tx))
    }

    pub(crate) fn set_result(&self, result: PyResult<PyObject>, aw: Py<PyFutureAwaitable>) {
        Python::with_gil(|py| {
            let mut state = self.state.write().unwrap();
            if !matches!(&mut *state, PyFutureAwaitableState::Pending) {
                return;
            }
            *state = PyFutureAwaitableState::Completed(result);

            let ack = self.ack.read().unwrap();
            if let Some((cb, ctx)) = &*ack {
                let _ = self.event_loop.clone_ref(py).call_method(
                    py,
                    pyo3::intern!(py, "call_soon_threadsafe"),
                    (cb, aw),
                    Some(ctx.bind(py)),
                );
            }
        });
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
        let state = pyself.state.read().unwrap();
        if let PyFutureAwaitableState::Completed(res) = &*state {
            let py = pyself.py();
            return res
                .as_ref()
                .map_err(|err| err.clone_ref(py))
                .map(|v| Err(PyStopIteration::new_err(v.clone_ref(py))))?;
        };
        drop(state);
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

    fn get_loop(&self, py: Python) -> PyObject {
        self.event_loop.clone_ref(py)
    }

    #[pyo3(signature = (cb, context=None))]
    fn add_done_callback(pyself: PyRef<'_, Self>, cb: PyObject, context: Option<PyObject>) -> PyResult<()> {
        let py = pyself.py();
        let kwctx = pyo3::types::PyDict::new(py);
        kwctx.set_item(pyo3::intern!(py, "context"), context)?;

        let state = pyself.state.read().unwrap();
        match &*state {
            PyFutureAwaitableState::Pending => {
                let mut ack = pyself.ack.write().unwrap();
                *ack = Some((cb, kwctx.unbind()));
                Ok(())
            }
            _ => {
                drop(state);
                let event_loop = pyself.event_loop.clone_ref(py);
                event_loop.call_method(py, pyo3::intern!(py, "call_soon"), (cb, pyself), Some(&kwctx))?;
                Ok(())
            }
        }
    }

    #[allow(unused)]
    fn remove_done_callback(&self, cb: PyObject) -> i32 {
        let mut ack = self.ack.write().unwrap();
        *ack = None;
        1
    }

    #[allow(unused)]
    #[pyo3(signature = (msg=None))]
    fn cancel(pyself: PyRef<'_, Self>, msg: Option<PyObject>) -> bool {
        let mut state = pyself.state.write().unwrap();
        if !matches!(&mut *state, PyFutureAwaitableState::Pending) {
            return false;
        }

        pyself.cancel_tx.notify_one();
        *state = PyFutureAwaitableState::Cancelled;

        let ack = pyself.ack.read().unwrap();
        if let Some((cb, ctx)) = &*ack {
            let py = pyself.py();
            let event_loop = pyself.event_loop.clone_ref(py);
            let cb = cb.clone_ref(py);
            let ctx = ctx.clone_ref(py);
            drop(ack);
            drop(state);

            let _ = event_loop.call_method(py, pyo3::intern!(py, "call_soon"), (cb, pyself), Some(ctx.bind(py)));
        }

        true
    }

    fn done(&self) -> bool {
        let state = self.state.read().unwrap();
        !matches!(&*state, PyFutureAwaitableState::Pending)
    }

    fn result(&self, py: Python) -> PyResult<PyObject> {
        let state = self.state.read().unwrap();
        match &*state {
            PyFutureAwaitableState::Completed(res) => {
                res.as_ref().map(|v| v.clone_ref(py)).map_err(|err| err.clone_ref(py))
            }
            PyFutureAwaitableState::Cancelled => {
                Err(pyo3::exceptions::asyncio::CancelledError::new_err("Future cancelled."))
            }
            PyFutureAwaitableState::Pending => Err(pyo3::exceptions::asyncio::InvalidStateError::new_err(
                "Result is not ready.",
            )),
        }
    }

    fn exception(&self, py: Python) -> PyResult<PyObject> {
        let state = self.state.read().unwrap();
        match &*state {
            PyFutureAwaitableState::Completed(res) => res.as_ref().map(|_| py.None()).map_err(|err| err.clone_ref(py)),
            PyFutureAwaitableState::Cancelled => {
                Err(pyo3::exceptions::asyncio::CancelledError::new_err("Future cancelled."))
            }
            PyFutureAwaitableState::Pending => Err(pyo3::exceptions::asyncio::InvalidStateError::new_err(
                "Exception is not set.",
            )),
        }
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
