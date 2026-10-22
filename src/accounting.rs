//! Lane-occupancy accounting.
//!
//! The occupancy of an async (ASGI/RSGI) request is measured from Python with
//! `sys.monitoring`: the callbacks registered here start a timer whenever the request's
//! root task step begins and stop it when the step suspends. WSGI requests are measured
//! directly in Rust, since a request holds its pool thread for the whole call.

use pyo3::{prelude::*, types::PyTuple};
use std::{
    cell::RefCell,
    sync::{Arc, Mutex, OnceLock, atomic},
    time::Instant,
};

use super::{callbacks::CallbackScheduler, runtime::RequestRt};

/// The accumulator shared by a request's protocol object and its Python context.
#[derive(Default)]
pub(crate) struct AcctState {
    ns: atomic::AtomicU64,
}

impl AcctState {
    fn add(&self, ns: u64) {
        self.ns.fetch_add(ns, atomic::Ordering::Relaxed);
    }

    fn take(&self) -> u64 {
        self.ns.swap(0, atomic::Ordering::Relaxed)
    }
}

/// Python-visible handle stored in a `ContextVar`, so that the monitoring callbacks can
/// look up which request is currently executing on the loop thread.
///
/// On async interfaces it is also how the request takes turns on the event loop: the
/// task driver in `granian._futures` queues a turn (`lane_wait`) before each step and gives
/// the lane back (`lane_release`) when the step suspends; `lane_dispatch` grants the next
/// turn. So the fair scheduler, not the loop's FIFO ready queue, decides which request's
/// step runs next.
#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct RequestAcct {
    pub(crate) state: Arc<AcctState>,
    rt: RequestRt,
}

impl RequestAcct {
    pub(crate) fn new(rt: RequestRt) -> Self {
        Self {
            state: Arc::new(AcctState::default()),
            rt,
        }
    }
}

#[pymethods]
impl RequestAcct {
    /// Queue for the loop lane; `fut` gets resolved once it is this request's turn.
    /// Returns `True` when the caller must schedule `lane_dispatch` with `call_soon`.
    fn lane_wait(&self, py: Python<'_>, fut: Py<PyAny>) -> PyResult<bool> {
        match self.rt.flow.as_ref() {
            Some(flow) => Ok(self.rt.rt.lane_enqueue(flow, fut)),
            // only tracked requests get an accounting handle, but never leave a waiter hanging
            None => {
                fut.call_method1(py, "set_result", (py.None(),))?;
                Ok(false)
            }
        }
    }

    /// The step suspended: charge its occupancy and give the lane back. Returns `True` when
    /// other requests are waiting and the caller must schedule `lane_dispatch`.
    fn lane_release(&self) -> bool {
        debit_state(&self.rt, &self.state);
        self.rt.rt.lane_release()
    }

    /// Grant the lane to the waiting turn with the highest remaining budget.
    fn lane_dispatch(&self, py: Python<'_>) -> PyResult<()> {
        while let Some(fut) = self.rt.rt.lane_next() {
            let fut = fut.bind(py);
            // a turn whose task got cancelled while waiting is skipped
            if fut.call_method0("done")?.is_truthy()? {
                continue;
            }
            fut.call_method1("set_result", (py.None(),))?;
            break;
        }
        Ok(())
    }
}

thread_local! {
    static CURRENT: RefCell<Option<(Arc<AcctState>, Instant)>> = const { RefCell::new(None) };
}

fn step_start(state: Arc<AcctState>) {
    CURRENT.with(|c| *c.borrow_mut() = Some((state, Instant::now())));
}

fn step_end() {
    CURRENT.with(|c| {
        if let Some((state, t0)) = c.borrow_mut().take() {
            state.add(t0.elapsed().as_nanos() as u64);
        }
    });
}

fn clear_current() {
    CURRENT.with(|c| {
        c.borrow_mut().take();
    });
}

/// Flush the currently running step (if it belongs to `state`) and return the total
/// occupancy accumulated since the previous debit.
fn take_current(state: &Arc<AcctState>) -> u64 {
    CURRENT.with(|c| {
        if let Some((current, t0)) = c.borrow_mut().as_mut()
            && Arc::ptr_eq(current, state)
        {
            let now = Instant::now();
            state.add((now - *t0).as_nanos() as u64);
            *t0 = now;
        }
    });
    state.take()
}

/// The context variable holding the current request's `RequestAcct`, exposed to Python so the
/// task driver can find the request's lane handle.
#[pyfunction]
pub(crate) fn request_acct_var(py: Python<'_>) -> Py<PyAny> {
    acct_var(py).clone_ref(py)
}

/// The context variable holding the current request's `RequestAcct`.
pub(crate) fn acct_var(py: Python<'_>) -> &'static Py<PyAny> {
    static VAR: OnceLock<Py<PyAny>> = OnceLock::new();
    VAR.get_or_init(|| {
        py.import("contextvars")
            .and_then(|m| m.getattr("ContextVar"))
            .and_then(|c| c.call1(("granian_request_acct",)))
            .expect("cannot create the request accounting context variable")
            .unbind()
    })
}

fn current_acct(py: Python<'_>) -> Option<Arc<AcctState>> {
    let var = acct_var(py).bind(py);
    let value = var.call_method1("get", (py.None(),)).ok()?;
    let acct = value.extract::<PyRef<'_, RequestAcct>>().ok()?;
    Some(acct.state.clone())
}

/// Charge everything the request consumed since its last debit to its flow.
pub(crate) fn debit(rt: &RequestRt, acct: Option<&Py<RequestAcct>>) {
    if let Some(acct) = acct {
        debit_state(rt, &acct.get().state);
    }
}

fn debit_state(rt: &RequestRt, state: &Arc<AcctState>) {
    if let Some(flow) = rt.flow.as_ref() {
        let cost = take_current(state);
        if cost > 0 {
            rt.rt.debit_flow(flow, cost);
        }
    }
}

/// Release the flow once the request is done (idempotent at the caller's level).
pub(crate) fn release(rt: &RequestRt) {
    if let Some(flow) = rt.flow {
        rt.rt.release_flow(flow);
    }
}

/// Build the per-request context for a task: a copy of the scheduler's base context with
/// the request's `RequestAcct` set.
#[allow(clippy::used_underscore_binding)]
pub(crate) fn build_context(
    py: Python<'_>,
    scheduler: &CallbackScheduler,
    acct: &Py<RequestAcct>,
) -> PyResult<Py<PyAny>> {
    // Contexts cannot be entered twice at once, and `_ctx` is shared by every request
    // of the worker, so serialize the short enter/copy/exit sequence.
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("context lock poisoned"))?;

    let contextvars = py.import("contextvars")?;
    let copy_context = contextvars.getattr("copy_context")?;
    let base = scheduler._ctx.bind(py);

    unsafe {
        pyo3::ffi::PyContext_Enter(base.as_ptr());
    }
    let copied = copy_context.call0();
    unsafe {
        pyo3::ffi::PyContext_Exit(base.as_ptr());
    }
    let copied = copied?;

    unsafe {
        pyo3::ffi::PyContext_Enter(copied.as_ptr());
    }
    let set = acct_var(py).bind(py).call_method1("set", (acct,));
    unsafe {
        pyo3::ffi::PyContext_Exit(copied.as_ptr());
    }
    set?;

    Ok(copied.unbind())
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct MonitoringStart;

#[pymethods]
impl MonitoringStart {
    #[pyo3(signature = (*args))]
    fn __call__(&self, args: &Bound<'_, PyTuple>) -> Py<PyAny> {
        let py = args.py();
        match current_acct(py) {
            Some(state) => step_start(state),
            None => clear_current(),
        }
        py.None()
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub(crate) struct MonitoringEnd;

#[pymethods]
impl MonitoringEnd {
    #[pyo3(signature = (*args))]
    fn __call__(&self, args: &Bound<'_, PyTuple>) -> Py<PyAny> {
        let _ = args;
        step_end();
        args.py().None()
    }
}

/// Create a per-request `RequestAcct` and the context that carries it, when the request is
/// tracked by the scheduler (`tracked` is then its runtime handle).
pub(crate) fn build_acct(
    py: Python<'_>,
    scheduler: &CallbackScheduler,
    tracked: Option<RequestRt>,
) -> (Option<Py<RequestAcct>>, Option<Py<PyAny>>) {
    let Some(rt) = tracked else {
        return (None, None);
    };
    match Py::new(py, RequestAcct::new(rt)) {
        Ok(acct) => match build_context(py, scheduler, &acct) {
            Ok(ctx) => (Some(acct), Some(ctx)),
            Err(err) => {
                log::warn!("Cannot build request context: {err:?}");
                (Some(acct), None)
            }
        },
        Err(err) => {
            log::warn!("Cannot allocate request accounting: {err:?}");
            (None, None)
        }
    }
}

/// Install the `sys.monitoring` hooks used to measure async lane occupancy. Only the code
/// object of `future_watcher` is instrumented, so no other code pays for monitoring.
pub(crate) fn install_monitoring(py: Python<'_>, scheduler: &CallbackScheduler, tool_id: u8) -> PyResult<()> {
    let monitoring = py.import("sys")?.getattr("monitoring")?;
    let events = monitoring.getattr("events")?;

    monitoring.call_method1("use_tool_id", (tool_id, "granian"))?;

    let start = Py::new(py, MonitoringStart)?;
    let end = Py::new(py, MonitoringEnd)?;
    for name in ["PY_START", "PY_RESUME", "PY_THROW"] {
        let event = events.getattr(name)?;
        monitoring.call_method1("register_callback", (tool_id, event, start.clone_ref(py)))?;
    }
    for name in ["PY_YIELD", "PY_RETURN", "PY_UNWIND"] {
        let event = events.getattr(name)?;
        monitoring.call_method1("register_callback", (tool_id, event, end.clone_ref(py)))?;
    }

    let code = scheduler.cb.getattr(py, "__code__")?;

    // Not every step event can be attached to a code object: CPython rejects e.g.
    // `PY_THROW` / `PY_UNWIND` as local events on some versions. Probe each event and
    // keep only the ones this interpreter accepts.
    let mut mask = 0u32;
    for name in [
        "PY_START",
        "PY_RESUME",
        "PY_THROW",
        "PY_YIELD",
        "PY_RETURN",
        "PY_UNWIND",
    ] {
        let event = events.getattr(name)?.extract::<u32>()?;
        if monitoring
            .call_method1("set_local_events", (tool_id, &code, event))
            .is_ok()
        {
            mask |= event;
        }
    }
    monitoring.call_method1("set_local_events", (tool_id, code, mask))?;

    Ok(())
}
