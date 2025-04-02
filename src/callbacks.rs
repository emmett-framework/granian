use pyo3::{exceptions::PyStopIteration, prelude::*, types::PyDict, IntoPyObjectExt};
use std::sync::{atomic, Arc, OnceLock, RwLock};
use tokio::sync::Notify;

use crate::{asyncio::copy_context, conversion::FutureResultToPy};

pub(crate) type ArcCBScheduler = Arc<Py<CallbackScheduler>>;

#[pyclass(frozen, subclass, module = "granian._granian")]
pub(crate) struct CallbackScheduler {
    pub cb: PyObject,
    #[pyo3(get)]
    _loop: PyObject,
    #[pyo3(get)]
    _ctx: PyObject,
    schedule_fn: OnceLock<PyObject>,
    aio_task: PyObject,
    aio_tenter: PyObject,
    aio_texit: PyObject,
    pym_lcs: PyObject,
    pyname_aioblock: PyObject,
    #[cfg(any(not(Py_3_10), PyPy))]
    pyname_aiosend: PyObject,
    pyname_aiothrow: PyObject,
    pyname_futcb: PyObject,
    pynone: PyObject,
    pyfalse: PyObject,
}

#[cfg(not(PyPy))]
impl CallbackScheduler {
    #[inline]
    pub(crate) fn schedule<T>(&self, py: Python, watcher: Py<T>) {
        let cbarg = watcher.as_ptr();
        let sched = self.schedule_fn.get().unwrap().as_ptr();

        unsafe {
            pyo3::ffi::PyObject_CallOneArg(sched, cbarg);
        }

        watcher.drop_ref(py);
    }

    #[cfg(Py_3_10)]
    #[inline]
    fn send(&self, py: Python, state: Arc<CallbackSchedulerState>) {
        let aiotask = self.aio_task.as_ptr();

        unsafe {
            pyo3::ffi::PyObject_CallOneArg(self.aio_tenter.as_ptr(), aiotask);

            let mut pres = std::ptr::null_mut::<pyo3::ffi::PyObject>();
            let gres = pyo3::ffi::PyIter_Send(state.coro.as_ptr(), self.pynone.as_ptr(), &mut pres);

            if gres == pyo3::ffi::PySendResult::PYGEN_NEXT {
                if pres == self.pynone.as_ptr() {
                    CallbackSchedulerState::reschedule(state, py, self.pym_lcs.as_ptr());
                } else {
                    let vptr = pyo3::ffi::PyObject_GetAttr(pres, self.pyname_aioblock.as_ptr());
                    if Bound::from_owned_ptr_or_err(py, vptr)
                        .map(|v| v.extract::<bool>().unwrap_or(false))
                        .unwrap_or(false)
                    {
                        pyo3::ffi::PyObject_SetAttr(pres, self.pyname_aioblock.as_ptr(), self.pyfalse.as_ptr());
                        CallbackSchedulerState::add_waker(state, py, pres, self.pyname_futcb.as_ptr());
                    }
                }
            }

            pyo3::ffi::PyObject_CallOneArg(self.aio_texit.as_ptr(), aiotask);
        }
    }

    #[cfg(not(Py_3_10))]
    #[inline]
    fn send(&self, py: Python, state: Arc<CallbackSchedulerState>) {
        let aiotask = self.aio_task.as_ptr();

        unsafe {
            pyo3::ffi::PyObject_CallOneArg(self.aio_tenter.as_ptr(), aiotask);
        }

        if let Ok(res) = unsafe {
            let pres = pyo3::ffi::PyObject_CallMethodOneArg(
                state.coro.as_ptr(),
                self.pyname_aiosend.as_ptr(),
                self.pynone.as_ptr(),
            );
            Bound::from_owned_ptr_or_err(py, pres)
        } {
            if unsafe {
                let vptr = pyo3::ffi::PyObject_GetAttr(res.as_ptr(), self.pyname_aioblock.as_ptr());
                Bound::from_owned_ptr_or_err(py, vptr)
                    .map(|v| v.extract::<bool>().unwrap_or(false))
                    .unwrap_or(false)
            } {
                let resp = res.as_ptr();
                unsafe {
                    pyo3::ffi::PyObject_SetAttr(resp, self.pyname_aioblock.as_ptr(), self.pyfalse.as_ptr());
                    CallbackSchedulerState::add_waker(state, py, resp, self.pyname_futcb.as_ptr());
                }
            } else {
                CallbackSchedulerState::reschedule(state, py, self.pym_lcs.as_ptr());
            }
        }

        unsafe {
            pyo3::ffi::PyObject_CallOneArg(self.aio_texit.as_ptr(), aiotask);
        }
    }

    #[inline]
    fn throw(&self, _py: Python, state: Arc<CallbackSchedulerState>, err: PyObject) {
        let aiotask = self.aio_task.as_ptr();

        unsafe {
            pyo3::ffi::PyObject_CallOneArg(self.aio_tenter.as_ptr(), aiotask);
            pyo3::ffi::PyObject_CallMethodOneArg(state.coro.as_ptr(), self.pyname_aiothrow.as_ptr(), err.into_ptr());
            pyo3::ffi::PyErr_Clear();
            pyo3::ffi::PyObject_CallOneArg(self.aio_texit.as_ptr(), aiotask);
        }
    }
}

#[cfg(PyPy)]
impl CallbackScheduler {
    #[inline]
    pub(crate) fn schedule<T>(&self, py: Python, watcher: Py<T>) {
        let cbarg = (watcher,).into_pyobject(py).unwrap().into_ptr();
        let sched = self.schedule_fn.get().unwrap().as_ptr();

        unsafe {
            pyo3::ffi::PyObject_CallObject(sched, cbarg);
        }
    }

    #[inline]
    fn send(&self, py: Python, state: Arc<CallbackSchedulerState>) {
        let aiotask = self.aio_task.as_ptr();

        unsafe {
            pyo3::ffi::PyObject_CallObject(self.aio_tenter.as_ptr(), aiotask);
        }

        if let Ok(res) = unsafe {
            let res = pyo3::ffi::PyObject_CallMethodObjArgs(
                state.coro.as_ptr(),
                self.pyname_aiosend.as_ptr(),
                self.pynone.as_ptr(),
                std::ptr::null_mut::<PyObject>(),
            );
            Bound::from_owned_ptr_or_err(py, res)
        } {
            if unsafe {
                let vptr = pyo3::ffi::PyObject_GetAttr(res.as_ptr(), self.pyname_aioblock.as_ptr());
                Bound::from_owned_ptr_or_err(py, vptr)
                    .map(|v| v.extract::<bool>().unwrap_or(false))
                    .unwrap_or(false)
            } {
                let resp = res.as_ptr();

                unsafe {
                    pyo3::ffi::PyObject_SetAttr(resp, self.pyname_aioblock.as_ptr(), self.pyfalse.as_ptr());
                    CallbackSchedulerState::add_waker(state, py, resp, self.pyname_futcb.as_ptr());
                }
            } else {
                CallbackSchedulerState::reschedule(state, py, self.pym_lcs.as_ptr());
            }
        }

        unsafe {
            pyo3::ffi::PyObject_CallObject(self.aio_texit.as_ptr(), aiotask);
        }
    }

    #[inline]
    fn throw(&self, py: Python, state: Arc<CallbackSchedulerState>, err: PyObject) {
        let aiotask = self.aio_task.as_ptr();

        unsafe {
            pyo3::ffi::PyObject_CallObject(self.aio_tenter.as_ptr(), aiotask);
            pyo3::ffi::PyObject_CallMethodObjArgs(
                state.coro.as_ptr(),
                self.pyname_aiothrow.as_ptr(),
                (err,).into_py_any(py).unwrap().into_ptr(),
                std::ptr::null_mut::<PyObject>(),
            );
            pyo3::ffi::PyErr_Clear();
            pyo3::ffi::PyObject_CallObject(self.aio_texit.as_ptr(), aiotask);
        }
    }
}

#[pymethods]
impl CallbackScheduler {
    #[new]
    fn new(
        py: Python,
        event_loop: PyObject,
        cb: PyObject,
        aio_task: PyObject,
        aio_tenter: PyObject,
        aio_texit: PyObject,
    ) -> Self {
        let ctx = copy_context(py);
        let pym_lcs = event_loop.getattr(py, pyo3::intern!(py, "call_soon")).unwrap();

        Self {
            _loop: event_loop,
            _ctx: ctx,
            schedule_fn: OnceLock::new(),
            cb,
            #[cfg(not(PyPy))]
            aio_task,
            #[cfg(PyPy)]
            aio_task: (aio_task,).into_py_any(py).unwrap(),
            aio_tenter,
            aio_texit,
            pyfalse: false.into_py_any(py).unwrap(),
            pynone: py.None(),
            pym_lcs,
            pyname_aioblock: pyo3::intern!(py, "_asyncio_future_blocking").into_py_any(py).unwrap(),
            #[cfg(any(not(Py_3_10), PyPy))]
            pyname_aiosend: pyo3::intern!(py, "send").into_py_any(py).unwrap(),
            pyname_aiothrow: pyo3::intern!(py, "throw").into_py_any(py).unwrap(),
            pyname_futcb: pyo3::intern!(py, "add_done_callback").into_py_any(py).unwrap(),
        }
    }

    #[setter(_schedule_fn)]
    fn _set_schedule_fn(&self, val: PyObject) {
        self.schedule_fn.set(val).unwrap();
    }

    fn _run(pyself: Py<Self>, py: Python, coro: PyObject) {
        let ctx = copy_context(py);
        let state = Arc::new(CallbackSchedulerState {
            sched: pyself.clone_ref(py),
            coro,
            ctx: ctx.clone_ref(py),
        });

        unsafe {
            pyo3::ffi::PyContext_Enter(ctx.as_ptr());
        }

        pyself.get().send(py, state);

        unsafe {
            pyo3::ffi::PyContext_Exit(ctx.as_ptr());
        }
    }
}

pub(crate) struct CallbackSchedulerState {
    sched: Py<CallbackScheduler>,
    coro: PyObject,
    ctx: PyObject,
}

impl CallbackSchedulerState {
    unsafe fn add_waker(self: Arc<Self>, py: Python, fut: *mut pyo3::ffi::PyObject, fut_cbm: *mut pyo3::ffi::PyObject) {
        let waker = Py::new(py, CallbackSchedulerWaker { state: self.clone() }).unwrap();
        let ctxd = PyDict::new(py);
        ctxd.set_item(pyo3::intern!(py, "context"), self.ctx.clone_ref(py))
            .unwrap();

        pyo3::ffi::PyObject_Call(
            pyo3::ffi::PyObject_GetAttr(fut, fut_cbm),
            (waker,).into_py_any(py).unwrap().as_ptr(),
            ctxd.as_ptr(),
        );
    }

    fn reschedule(self: Arc<Self>, py: Python, loop_m: *mut pyo3::ffi::PyObject) {
        let step = Py::new(py, CallbackSchedulerStep { state: self.clone() }).unwrap();
        let ctxd = PyDict::new(py);
        ctxd.set_item(pyo3::intern!(py, "context"), self.ctx.clone_ref(py))
            .unwrap();

        unsafe {
            pyo3::ffi::PyObject_Call(loop_m, (step,).into_py_any(py).unwrap().as_ptr(), ctxd.as_ptr());
        }
    }
}

#[pyclass(frozen, module = "granian._granian")]
struct CallbackSchedulerWaker {
    state: Arc<CallbackSchedulerState>,
}

#[pymethods]
impl CallbackSchedulerWaker {
    fn __call__(&self, py: Python, fut: PyObject) {
        match fut.call_method0(py, pyo3::intern!(py, "result")) {
            Ok(_) => self.state.sched.get().send(py, self.state.clone()),
            Err(err) => self
                .state
                .sched
                .get()
                .throw(py, self.state.clone(), err.into_py_any(py).unwrap()),
        }
    }
}

#[pyclass(frozen, module = "granian._granian")]
struct CallbackSchedulerStep {
    state: Arc<CallbackSchedulerState>,
}

#[pymethods]
impl CallbackSchedulerStep {
    fn __call__(&self, py: Python) {
        self.state.sched.get().send(py, self.state.clone());
    }
}

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
    result: PyResult<PyObject>,
}

impl PyDoneAwaitable {
    pub(crate) fn new(result: PyResult<PyObject>) -> Self {
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

    fn __next__(&self, py: Python) -> PyResult<PyObject> {
        self.result
            .as_ref()
            .map(|v| v.clone_ref(py))
            .map_err(|v| v.clone_ref(py))
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
    result: OnceLock<PyResult<PyObject>>,
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

    fn __next__(&self, py: Python) -> PyResult<Option<PyObject>> {
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
    result: OnceLock<PyResult<PyObject>>,
    event_loop: PyObject,
    cancel_tx: Arc<Notify>,
    cancel_msg: OnceLock<PyObject>,
    py_block: atomic::AtomicBool,
    ack: RwLock<Option<(PyObject, Py<pyo3::types::PyDict>)>>,
}

impl PyFutureAwaitable {
    pub(crate) fn new(event_loop: PyObject) -> Self {
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

        {
            let ack = rself.ack.read().unwrap();
            if let Some((cb, ctx)) = &*ack {
                _ = rself.event_loop.clone_ref(py).call_method(
                    py,
                    pyo3::intern!(py, "call_soon_threadsafe"),
                    (cb, pyself.clone_ref(py)),
                    Some(ctx.bind(py)),
                );
            }
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

    fn get_loop(&self, py: Python) -> PyObject {
        self.event_loop.clone_ref(py)
    }

    #[pyo3(signature = (cb, context=None))]
    fn add_done_callback(pyself: PyRef<'_, Self>, cb: PyObject, context: Option<PyObject>) -> PyResult<()> {
        let py = pyself.py();
        let kwctx = pyo3::types::PyDict::new(py);
        kwctx.set_item(pyo3::intern!(py, "context"), context)?;

        let state = pyself.state.load(atomic::Ordering::Acquire);
        if state == PyFutureAwaitableState::Pending as u8 {
            let mut ack = pyself.ack.write().unwrap();
            *ack = Some((cb, kwctx.unbind()));
        } else {
            let event_loop = pyself.event_loop.clone_ref(py);
            event_loop.call_method(py, pyo3::intern!(py, "call_soon"), (cb, pyself), Some(&kwctx))?;
        }

        Ok(())
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

    fn result(&self, py: Python) -> PyResult<PyObject> {
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

    fn exception(&self, py: Python) -> PyResult<PyObject> {
        let state = self.state.load(atomic::Ordering::Acquire);

        if state == PyFutureAwaitableState::Completed as u8 {
            return self
                .result
                .get()
                .unwrap()
                .as_ref()
                .map(|_| py.None())
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
