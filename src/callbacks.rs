#[cfg(Py_3_12)]
use std::cell::RefCell;

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

    #[cfg(Py_3_12)]
    #[inline]
    pub(crate) fn send(pyself: Py<Self>, py: Python, state: Py<CallbackSchedulerState>) {
        let rself = pyself.get();
        let rstate = state.borrow(py);
        let aiotask = rself.aio_task.as_ptr();

        *rstate.futw.borrow_mut() = None;

        unsafe {
            pyo3::ffi::PyObject_CallOneArg(rself.aio_tenter.as_ptr(), aiotask);
        }

        if let Some(res) = unsafe {
            let mut pres = std::ptr::null_mut::<pyo3::ffi::PyObject>();
            // FIXME: use PyIter_Send return value once available in PyO3
            pyo3::ffi::PyIter_Send(rstate.coro.as_ptr(), rself.pynone.as_ptr(), &mut pres);
            Bound::from_owned_ptr_or_opt(py, pres)
                .map(|v| {
                    if v.is_none() {
                        return None;
                    }
                    Some(v)
                })
                .unwrap()
        } {
            if unsafe {
                let vptr = pyo3::ffi::PyObject_GetAttr(res.as_ptr(), rself.pyname_aioblock.as_ptr());
                Bound::from_owned_ptr_or_err(py, vptr)
                    .map(|v| v.extract::<bool>().unwrap_or(false))
                    .unwrap_or(false)
            } {
                let resp = res.as_ptr();
                *rstate.futw.borrow_mut() = Some(res.unbind().clone_ref(py));
                drop(rstate);

                unsafe {
                    pyo3::ffi::PyObject_SetAttr(resp, rself.pyname_aioblock.as_ptr(), rself.pyfalse.as_ptr());
                    CallbackSchedulerState::schedule(state, py, resp);
                }
            } else {
                drop(rstate);
                CallbackSchedulerState::reschedule(state, py);
            }
        } else {
            drop(rstate);
            state.drop_ref(py);
        }

        unsafe {
            pyo3::ffi::PyObject_CallOneArg(rself.aio_texit.as_ptr(), aiotask);
        }
    }

    #[cfg(all(not(Py_3_12), Py_3_10))]
    #[inline]
    pub(crate) fn send(pyself: Py<Self>, py: Python, state: Py<CallbackSchedulerState>) {
        let rself = pyself.get();
        let aiotask = rself.aio_task.as_ptr();

        unsafe {
            pyo3::ffi::PyObject_CallOneArg(rself.aio_tenter.as_ptr(), aiotask);
        }

        if let Some(res) = unsafe {
            let mut pres = std::ptr::null_mut::<pyo3::ffi::PyObject>();
            // FIXME: use PyIter_Send return value once available in PyO3
            pyo3::ffi::PyIter_Send(state.borrow(py).coro.as_ptr(), rself.pynone.as_ptr(), &mut pres);
            Bound::from_owned_ptr_or_opt(py, pres)
                .map(|v| {
                    if v.is_none() {
                        return None;
                    }
                    Some(v)
                })
                .unwrap()
        } {
            if unsafe {
                let vptr = pyo3::ffi::PyObject_GetAttr(res.as_ptr(), rself.pyname_aioblock.as_ptr());
                Bound::from_owned_ptr_or_err(py, vptr)
                    .map(|v| v.extract::<bool>().unwrap_or(false))
                    .unwrap_or(false)
            } {
                let resp = res.as_ptr();
                unsafe {
                    pyo3::ffi::PyObject_SetAttr(resp, rself.pyname_aioblock.as_ptr(), rself.pyfalse.as_ptr());
                    CallbackSchedulerState::schedule(state, py, resp);
                }
            } else {
                CallbackSchedulerState::reschedule(state, py);
            }
        } else {
            state.drop_ref(py);
        }

        unsafe {
            pyo3::ffi::PyObject_CallOneArg(rself.aio_texit.as_ptr(), aiotask);
        }
    }

    #[cfg(not(Py_3_10))]
    #[inline]
    pub(crate) fn send(pyself: Py<Self>, py: Python, state: Py<CallbackSchedulerState>) {
        let rself = pyself.get();
        let aiotask = rself.aio_task.as_ptr();

        unsafe {
            pyo3::ffi::PyObject_CallOneArg(rself.aio_tenter.as_ptr(), aiotask);
        }

        if let Ok(res) = unsafe {
            let pres = pyo3::ffi::PyObject_CallMethodOneArg(
                state.borrow(py).coro.as_ptr(),
                rself.pyname_aiosend.as_ptr(),
                rself.pynone.as_ptr(),
            );
            Bound::from_owned_ptr_or_err(py, pres)
        } {
            if unsafe {
                let vptr = pyo3::ffi::PyObject_GetAttr(res.as_ptr(), rself.pyname_aioblock.as_ptr());
                Bound::from_owned_ptr_or_err(py, vptr)
                    .map(|v| v.extract::<bool>().unwrap_or(false))
                    .unwrap_or(false)
            } {
                let resp = res.as_ptr();
                unsafe {
                    pyo3::ffi::PyObject_SetAttr(resp, rself.pyname_aioblock.as_ptr(), rself.pyfalse.as_ptr());
                    CallbackSchedulerState::schedule(state, py, resp);
                }
            } else {
                CallbackSchedulerState::reschedule(state, py);
            }
        } else {
            state.drop_ref(py);
        }

        unsafe {
            pyo3::ffi::PyObject_CallOneArg(rself.aio_texit.as_ptr(), aiotask);
        }
    }

    #[inline]
    pub(crate) fn throw(pyself: Py<Self>, py: Python, state: Py<CallbackSchedulerState>, err: PyObject) {
        let rself = pyself.get();
        let aiotask = rself.aio_task.as_ptr();

        unsafe {
            pyo3::ffi::PyObject_CallOneArg(rself.aio_tenter.as_ptr(), aiotask);
            pyo3::ffi::PyObject_CallMethodOneArg(
                state.borrow(py).coro.as_ptr(),
                rself.pyname_aiothrow.as_ptr(),
                err.into_ptr(),
            );
            pyo3::ffi::PyErr_Clear();
            pyo3::ffi::PyObject_CallOneArg(rself.aio_texit.as_ptr(), aiotask);
        }

        state.drop_ref(py);
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
    pub(crate) fn send(pyself: Py<Self>, py: Python, state: Py<CallbackSchedulerState>) {
        let rself = pyself.get();
        let aiotask = rself.aio_task.as_ptr();

        unsafe {
            pyo3::ffi::PyObject_CallObject(rself.aio_tenter.as_ptr(), aiotask);
        }

        if let Ok(res) = unsafe {
            let res = pyo3::ffi::PyObject_CallMethodObjArgs(
                state.borrow(py).coro.as_ptr(),
                rself.pyname_aiosend.as_ptr(),
                rself.pynone.as_ptr(),
                std::ptr::null_mut::<PyObject>(),
            );
            Bound::from_owned_ptr_or_err(py, res)
        } {
            if unsafe {
                let vptr = pyo3::ffi::PyObject_GetAttr(res.as_ptr(), rself.pyname_aioblock.as_ptr());
                Bound::from_owned_ptr_or_err(py, vptr)
                    .map(|v| v.extract::<bool>().unwrap_or(false))
                    .unwrap_or(false)
            } {
                let resp = res.as_ptr();

                unsafe {
                    pyo3::ffi::PyObject_SetAttr(resp, rself.pyname_aioblock.as_ptr(), rself.pyfalse.as_ptr());
                    CallbackSchedulerState::schedule(state, py, resp);
                }
            } else {
                CallbackSchedulerState::reschedule(state, py);
            }
        } else {
            state.drop_ref(py);
        }

        unsafe {
            pyo3::ffi::PyObject_CallObject(rself.aio_texit.as_ptr(), aiotask);
        }
    }

    #[inline]
    pub(crate) fn throw(pyself: Py<Self>, py: Python, state: Py<CallbackSchedulerState>, err: PyObject) {
        let rself = pyself.get();
        let aiotask = rself.aio_task.as_ptr();

        unsafe {
            pyo3::ffi::PyObject_CallObject(rself.aio_tenter.as_ptr(), aiotask);
            pyo3::ffi::PyObject_CallMethodObjArgs(
                state.borrow(py).coro.as_ptr(),
                rself.pyname_aiothrow.as_ptr(),
                (err,).into_py_any(py).unwrap().into_ptr(),
                std::ptr::null_mut::<PyObject>(),
            );
            pyo3::ffi::PyErr_Clear();
            pyo3::ffi::PyObject_CallObject(rself.aio_texit.as_ptr(), aiotask);
        }

        state.drop_ref(py);
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
        }
    }

    #[setter(_schedule_fn)]
    fn _set_schedule_fn(&self, val: PyObject) {
        self.schedule_fn.set(val).unwrap();
    }

    fn _run(pyself: Py<Self>, py: Python, coro: PyObject) {
        let ctx = copy_context(py);
        let state = CallbackSchedulerState::new(
            py,
            pyself.clone_ref(py),
            coro,
            ctx.clone_ref(py),
            pyself.get().pym_lcs.clone_ref(py),
        );

        unsafe {
            pyo3::ffi::PyContext_Enter(ctx.as_ptr());
        }

        CallbackScheduler::send(pyself, py, state);

        unsafe {
            pyo3::ffi::PyContext_Exit(ctx.as_ptr());
        }
    }
}

#[pyclass(frozen, freelist = 1024, unsendable, module = "granian._granian")]
pub(crate) struct CallbackSchedulerState {
    sched: Py<CallbackScheduler>,
    coro: PyObject,
    ctxd: Py<PyDict>,
    pys_futcb: PyObject,
    pym_schedule: PyObject,
    #[cfg(Py_3_12)]
    futw: RefCell<Option<PyObject>>,
}

impl CallbackSchedulerState {
    fn new(
        py: Python,
        sched: Py<CallbackScheduler>,
        coro: PyObject,
        ctx: PyObject,
        pym_schedule: PyObject,
    ) -> Py<Self> {
        let ctxd = PyDict::new(py);
        ctxd.set_item(pyo3::intern!(py, "context"), ctx).unwrap();

        Py::new(
            py,
            Self {
                sched,
                coro,
                ctxd: ctxd.unbind(),
                pys_futcb: pyo3::intern!(py, "add_done_callback").into_py_any(py).unwrap(),
                pym_schedule,
                #[cfg(Py_3_12)]
                futw: RefCell::new(None),
            },
        )
        .unwrap()
    }

    unsafe fn schedule(pyself: Py<Self>, py: Python, step: *mut pyo3::ffi::PyObject) {
        let rself = pyself.borrow(py);
        pyo3::ffi::PyObject_Call(
            pyo3::ffi::PyObject_GetAttr(step, rself.pys_futcb.as_ptr()),
            pyself.getattr(py, pyo3::intern!(py, "wake")).unwrap().as_ptr(),
            rself.ctxd.as_ptr(),
        );
    }

    fn reschedule(pyself: Py<Self>, py: Python) {
        let rself = pyself.borrow(py);
        unsafe {
            pyo3::ffi::PyObject_Call(rself.pym_schedule.as_ptr(), pyself.as_ptr(), rself.ctxd.as_ptr());
        }
    }
}

#[pymethods]
impl CallbackSchedulerState {
    fn __call__(pyself: Py<Self>, py: Python) {
        let sched = pyself.borrow(py).sched.clone_ref(py);
        CallbackScheduler::send(sched, py, pyself);
    }

    fn wake(pyself: Py<Self>, py: Python, fut: PyObject) {
        let sched = pyself.borrow(py).sched.clone_ref(py);
        match fut.call_method0(py, pyo3::intern!(py, "result")) {
            Ok(_) => CallbackScheduler::send(sched, py, pyself),
            Err(err) => CallbackScheduler::throw(sched, py, pyself, err.into_py_any(py).unwrap()),
        }
    }

    #[cfg(Py_3_12)]
    fn cancel(&self, py: Python) -> PyResult<PyObject> {
        if let Some(v) = self.futw.borrow().as_ref() {
            return v.call_method0(py, pyo3::intern!(py, "cancel"));
        }
        Ok(self.sched.get().pyfalse.clone_ref(py))
    }

    #[cfg(Py_3_12)]
    fn cancelling(&self) -> i32 {
        0
    }

    #[cfg(Py_3_12)]
    fn uncancel(&self) -> i32 {
        0
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
            return Err(pyo3::exceptions::asyncio::CancelledError::new_err("Future cancelled."));
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
            return Err(pyo3::exceptions::asyncio::CancelledError::new_err("Future cancelled."));
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
