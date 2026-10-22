//! Event scheduling between the Rust side and the Python side.
//!
//! Every [`BlockingTask`] queued for a Python thread goes through a [`Scheduler`].
//! [`FifoScheduler`] is Granian's historical FIFO queue. [`FairScheduler`] is a two-level
//! fair queue keyed by client fingerprint (IP prefix, then browser fingerprint), so that a
//! single abusive or slow client cannot starve the others sharing a worker.

use crossbeam_channel as channel;
use hyper::{HeaderMap, header::HeaderName};
use pyo3::prelude::*;
use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    hash::{BuildHasher, Hasher},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};

use super::{blocking::BlockingTask, net::SockAddr};

/// Tasks that could not be pushed because the queue is closed.
#[derive(Debug)]
pub(crate) struct PushError;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PopError {
    Closed,
    Timeout,
}

/// Identifies the flow a request belongs to: one client fingerprint (inner) inside
/// one IP prefix (outer).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub(crate) struct FlowRef {
    pub outer: OuterKey,
    pub inner: u64,
}

/// An IP prefix. IPv4 addresses are stored in the low 32 bits of `masked`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub(crate) struct OuterKey {
    pub family: u8,
    pub masked: u128,
}

const OVERFLOW_OUTER: OuterKey = OuterKey { family: 0, masked: 0 };
const INNER_OVERFLOW: u64 = u64::MAX;

pub(crate) trait Scheduler: Send + Sync + 'static {
    /// Once per request, in `Service::call`. `None` = request not tracked.
    fn resolve(&self, peer: &SockAddr, headers: &HeaderMap) -> Option<FlowRef>;
    /// Request ended: done/err, end of WSGI task, WebSocket closed.
    fn release(&self, flow: FlowRef);
    /// Charge lane occupancy (ns) to the flow.
    fn debit(&self, flow: &FlowRef, cost_ns: u64);

    fn push(&self, task: BlockingTask, flow: Option<&FlowRef>) -> Result<(), PushError>;
    fn pop(&self) -> Option<BlockingTask>;
    fn pop_timeout(&self, d: Duration) -> Result<BlockingTask, PopError>;
    fn len(&self) -> usize;

    /// Queue a turn on the event-loop lane for a step of a tracked request (`fut` is returned
    /// by `lane_next` once it is granted); `true` if the caller must schedule a dispatch.
    fn lane_enqueue(&self, _flow: &FlowRef, _fut: Py<PyAny>) -> bool {
        false
    }
    /// Pick the next turn to grant, if any (or let the lane go idle).
    fn lane_next(&self) -> Option<Py<PyAny>> {
        None
    }
    /// A granted step ended; `true` if the caller must schedule a dispatch.
    fn lane_release(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------------------------
// FIFO
// ---------------------------------------------------------------------------------------------

pub(crate) struct FifoScheduler {
    tx: channel::Sender<BlockingTask>,
    rx: channel::Receiver<BlockingTask>,
}

impl FifoScheduler {
    pub fn new() -> Self {
        let (tx, rx) = channel::unbounded();
        Self { tx, rx }
    }
}

impl Scheduler for FifoScheduler {
    #[inline]
    fn resolve(&self, _peer: &SockAddr, _headers: &HeaderMap) -> Option<FlowRef> {
        None
    }

    #[inline]
    fn release(&self, _flow: FlowRef) {}

    #[inline]
    fn debit(&self, _flow: &FlowRef, _cost_ns: u64) {}

    #[inline]
    fn push(&self, task: BlockingTask, _flow: Option<&FlowRef>) -> Result<(), PushError> {
        self.tx.send(task).map_err(|_| PushError)
    }

    #[inline]
    fn pop(&self) -> Option<BlockingTask> {
        self.rx.recv().ok()
    }

    #[inline]
    fn pop_timeout(&self, d: Duration) -> Result<BlockingTask, PopError> {
        match self.rx.recv_timeout(d) {
            Ok(task) => Ok(task),
            Err(channel::RecvTimeoutError::Timeout) => Err(PopError::Timeout),
            Err(channel::RecvTimeoutError::Disconnected) => Err(PopError::Closed),
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.rx.len()
    }
}

// ---------------------------------------------------------------------------------------------
// Fair queuing
// ---------------------------------------------------------------------------------------------

/// The fair queue serves two independent lanes, sharing the flows' budgets.
///
/// * `Events`: the tasks pushed by Rust for the Python thread(s), as for every scheduler.
/// * `Steps`: on async interfaces, the event loop itself. A tracked request must hold it to
///   run a step of its task, and hands it back when the step suspends. Without it, every
///   admitted request would be an ordinary asyncio task, stepped in FIFO order by the loop's
///   ready queue, and the fair queue would only decide in which order requests *start*.
#[derive(Clone, Copy)]
enum Lane {
    Events = 0,
    Steps = 1,
}

const LANES: usize = 2;

/// What a flow queues on a lane: events on `Lane::Events`, turns on `Lane::Steps`.
enum Item {
    Task(BlockingTask),
    Turn(Turn),
}

/// A request waiting for the loop: the future to resolve once it is granted the lane.
///
/// Turns can be dropped off the loop thread, e.g. with the scheduler at worker shutdown, and
/// releasing a Python reference requires being attached to the interpreter. Attach if
/// possible, otherwise (interpreter finalizing) leak the reference rather than abort.
struct Turn(std::mem::ManuallyDrop<Py<PyAny>>);

impl Turn {
    fn new(fut: Py<PyAny>) -> Self {
        Self(std::mem::ManuallyDrop::new(fut))
    }

    fn into_inner(mut self) -> Py<PyAny> {
        // SAFETY: the value is taken exactly once, and `self` is forgotten right after.
        let fut = unsafe { std::mem::ManuallyDrop::take(&mut self.0) };
        std::mem::forget(self);
        fut
    }
}

impl Drop for Turn {
    fn drop(&mut self) {
        // SAFETY: `drop` runs at most once and `into_inner` forgets `self`.
        let mut fut = Some(unsafe { std::mem::ManuallyDrop::take(&mut self.0) });
        if Python::try_attach(|_| drop(fut.take())).is_none() {
            std::mem::forget(fut);
        }
    }
}

struct InnerFlow {
    used: u64,
    in_flight: usize,
    queued: [VecDeque<Item>; LANES],
    last_active: Instant,
}

impl InnerFlow {
    fn new(v: u64, burst: u64) -> Self {
        Self {
            used: v.saturating_sub(burst),
            in_flight: 0,
            queued: [VecDeque::new(), VecDeque::new()],
            last_active: Instant::now(),
        }
    }

    fn idle(&self) -> bool {
        self.queued.iter().all(VecDeque::is_empty)
    }
}

struct OuterFlow {
    used: u64,
    v: u64,
    in_flight: usize,
    inners: HashMap<u64, InnerFlow>,
    backlog: [BTreeSet<(u64, u64)>; LANES],
    last_active: Instant,
}

impl OuterFlow {
    fn new(v: u64, burst: u64) -> Self {
        Self {
            used: v.saturating_sub(burst),
            v,
            in_flight: 0,
            inners: HashMap::new(),
            backlog: [BTreeSet::new(), BTreeSet::new()],
            last_active: Instant::now(),
        }
    }

    fn backlogged(&self) -> bool {
        self.backlog.iter().any(|b| !b.is_empty())
    }
}

/// Move an inner flow to its new `used` position in every lane it is backlogged on.
fn rekey_inner(backlog: &mut [BTreeSet<(u64, u64)>; LANES], inner: &InnerFlow, ikey: u64, old: u64) {
    if inner.used == old {
        return;
    }
    for (lane, set) in backlog.iter_mut().enumerate() {
        if !inner.queued[lane].is_empty() {
            set.remove(&(old, ikey));
            set.insert((inner.used, ikey));
        }
    }
}

/// Move an outer flow to its new `used` position in every lane it is backlogged on.
fn rekey_outer(root: &mut [BTreeSet<(u64, OuterKey)>; LANES], o: &OuterFlow, okey: OuterKey, old: u64) {
    if o.used == old {
        return;
    }
    for (lane, set) in root.iter_mut().enumerate() {
        if !o.backlog[lane].is_empty() {
            set.remove(&(old, okey));
            set.insert((o.used, okey));
        }
    }
}

struct FairQueue {
    v: u64,
    outers: HashMap<OuterKey, OuterFlow>,
    root_backlog: [BTreeSet<(u64, OuterKey)>; LANES],
    system: VecDeque<BlockingTask>,
    /// Queued events (system + `Lane::Events`); the length seen by the blocking pool.
    queued: usize,
    inner_count: usize,
    overflow_outer: Option<OuterKey>,
    /// Whether some request currently holds (or has been granted) the loop.
    lane_busy: bool,
    cfg: FairConfigInner,
    closed: bool,
}

#[derive(Clone)]
struct FairConfigInner {
    outer_prefix_v4: u8,
    outer_prefix_v6: u8,
    outer_max: usize,
    inner_max_per_outer: usize,
    inner_max: usize,
    burst: u64,
    max_debt: u64,
    debt_ttl: Duration,
    trusted_proxies: Arc<TrustedProxies>,
    client_ip_header: HeaderName,
    inner_headers: Vec<String>,
    inner_cookies: Vec<String>,
    inner_include_ip: bool,
}

impl FairQueue {
    fn new(cfg: FairConfigInner) -> Self {
        Self {
            v: 0,
            outers: HashMap::new(),
            root_backlog: [BTreeSet::new(), BTreeSet::new()],
            system: VecDeque::new(),
            queued: 0,
            inner_count: 0,
            overflow_outer: None,
            lane_busy: false,
            cfg,
            closed: false,
        }
    }

    #[inline]
    fn budget_pos(level_v: u64, used: u64) -> bool {
        level_v >= used
    }

    fn push(&mut self, task: BlockingTask, flow: Option<FlowRef>) {
        match flow {
            Some(flow) => self.push_item(Lane::Events, Item::Task(task), flow),
            None => self.system.push_back(task),
        }
        self.queued += 1;
    }

    fn push_item(&mut self, lane: Lane, item: Item, flow: FlowRef) {
        let FlowRef {
            outer: okey,
            inner: ikey,
        } = flow;
        let lane = lane as usize;
        let v = self.v;
        let burst = self.cfg.burst;
        let now = Instant::now();

        let o = self.outers.entry(okey).or_insert_with(|| OuterFlow::new(v, burst));
        o.last_active = now;
        let ov = o.v;
        let outer_backlogged = !o.backlog[lane].is_empty();

        let inner = o.inners.entry(ikey).or_insert_with(|| InnerFlow::new(ov, burst));
        inner.last_active = now;
        if inner.queued[lane].is_empty() {
            let old = inner.used;
            inner.used = inner.used.max(ov.saturating_sub(burst));
            rekey_inner(&mut o.backlog, inner, ikey, old);
            o.backlog[lane].insert((inner.used, ikey));
        }
        inner.queued[lane].push_back(item);

        if !outer_backlogged {
            let old = o.used;
            o.used = o.used.max(v.saturating_sub(burst));
            rekey_outer(&mut self.root_backlog, o, okey, old);
            self.root_backlog[lane].insert((o.used, okey));
        }
    }

    fn pop_task(&mut self) -> Option<BlockingTask> {
        if let Some(task) = self.system.pop_front() {
            self.queued -= 1;
            return Some(task);
        }
        match self.pop_item(Lane::Events)? {
            Item::Task(task) => {
                self.queued -= 1;
                Some(task)
            }
            // never pushed on the events lane
            Item::Turn(_) => None,
        }
    }

    fn pop_item(&mut self, lane: Lane) -> Option<Item> {
        let lane = lane as usize;

        // root level: backlogged outer with the highest remaining budget (= lowest `used`)
        let (o_used, okey) = *self.root_backlog[lane].iter().next()?;
        self.v = self.v.max(o_used);

        let o = self.outers.get_mut(&okey)?;
        let (i_used, ikey) = *o.backlog[lane].iter().next()?;
        o.v = o.v.max(i_used);

        let inner = o.inners.get_mut(&ikey)?;
        let item = inner.queued[lane].pop_front()?;

        if inner.queued[lane].is_empty() {
            o.backlog[lane].remove(&(i_used, ikey));
        }
        if o.backlog[lane].is_empty() {
            self.root_backlog[lane].remove(&(o_used, okey));
        }

        Some(item)
    }

    // -- loop lane -----------------------------------------------------------------------------
    //
    // The lane is busy from the moment a dispatch is pending until the granted step ends.
    // Waiting turns are never granted on the spot: the caller schedules a dispatch with
    // `call_soon`, which runs after every wakeup already in the loop's ready queue, so that
    // the choice is made among all the requests that are ready to run, not just the first.

    /// Queue a turn; `true` if the lane was idle and the caller must schedule a dispatch.
    fn lane_enqueue(&mut self, flow: FlowRef, fut: Py<PyAny>) -> bool {
        self.push_item(Lane::Steps, Item::Turn(Turn::new(fut)), flow);
        !std::mem::replace(&mut self.lane_busy, true)
    }

    /// Dispatch: the turn with the highest remaining budget, or `None` (the lane goes idle).
    fn lane_next(&mut self) -> Option<Py<PyAny>> {
        if let Some(Item::Turn(turn)) = self.pop_item(Lane::Steps) {
            return Some(turn.into_inner());
        }
        self.lane_busy = false;
        None
    }

    /// The granted step ended; `true` if turns are waiting and a dispatch must be scheduled.
    fn lane_release(&mut self) -> bool {
        if self.root_backlog[Lane::Steps as usize].is_empty() {
            self.lane_busy = false;
            return false;
        }
        true
    }

    fn debit(&mut self, flow: &FlowRef, cost: u64) {
        let v = self.v;
        let max_debt = self.cfg.max_debt;
        let now = Instant::now();

        let Some(o) = self.outers.get_mut(&flow.outer) else {
            return;
        };
        let ov = o.v;

        if let Some(inner) = o.inners.get_mut(&flow.inner) {
            let old = inner.used;
            inner.used = inner.used.saturating_add(cost).min(ov.saturating_add(max_debt));
            inner.last_active = now;
            rekey_inner(&mut o.backlog, inner, flow.inner, old);
        }

        let old = o.used;
        o.used = o.used.saturating_add(cost).min(v.saturating_add(max_debt));
        o.last_active = now;
        rekey_outer(&mut self.root_backlog, o, flow.outer, old);
    }

    fn release(&mut self, flow: FlowRef) {
        let now = Instant::now();
        let ttl = self.cfg.debt_ttl;
        let mut remove_outer = false;

        if let Some(o) = self.outers.get_mut(&flow.outer) {
            o.in_flight = o.in_flight.saturating_sub(1);
            let outer_v = o.v;
            if let Some(inner) = o.inners.get_mut(&flow.inner) {
                inner.in_flight = inner.in_flight.saturating_sub(1);
            }

            let drop_inner = o.inners.get(&flow.inner).is_some_and(|inner| {
                inner.idle()
                    && inner.in_flight == 0
                    && (Self::budget_pos(outer_v, inner.used) || now.duration_since(inner.last_active) > ttl)
            });
            if drop_inner && let Some(inner) = o.inners.remove(&flow.inner) {
                self.inner_count = self.inner_count.saturating_sub(1);
                for set in &mut o.backlog {
                    set.remove(&(inner.used, flow.inner));
                }
            }

            if o.inners.is_empty()
                && o.in_flight == 0
                && !o.backlogged()
                && (Self::budget_pos(self.v, o.used) || now.duration_since(o.last_active) > ttl)
            {
                remove_outer = true;
            }
        }

        if remove_outer {
            self.outers.remove(&flow.outer);
            if self.overflow_outer == Some(flow.outer) {
                self.overflow_outer = None;
            }
        }
    }

    fn gc(&mut self) {
        let now = Instant::now();
        let v = self.v;
        let ttl = self.cfg.debt_ttl;

        let mut dead_outers = Vec::new();
        for (okey, o) in &mut self.outers {
            let mut dead_inners = Vec::new();
            for (ikey, inner) in &o.inners {
                if inner.idle()
                    && inner.in_flight == 0
                    && (Self::budget_pos(o.v, inner.used) || now.duration_since(inner.last_active) > ttl)
                {
                    dead_inners.push(*ikey);
                }
            }
            for ikey in dead_inners {
                if let Some(inner) = o.inners.remove(&ikey) {
                    self.inner_count = self.inner_count.saturating_sub(1);
                    for set in &mut o.backlog {
                        set.remove(&(inner.used, ikey));
                    }
                }
            }

            if o.inners.is_empty()
                && o.in_flight == 0
                && !o.backlogged()
                && (Self::budget_pos(v, o.used) || now.duration_since(o.last_active) > ttl)
            {
                dead_outers.push(*okey);
            }
        }

        for okey in dead_outers {
            self.outers.remove(&okey);
            if self.overflow_outer == Some(okey) {
                self.overflow_outer = None;
            }
        }
    }

    // -- flow identification -------------------------------------------------------------------

    fn client_ip(&self, peer: &SockAddr, headers: &HeaderMap) -> Option<IpAddr> {
        let peer_ip = match peer {
            SockAddr::TCP(addr) => Some(normalize_ip(addr.ip())),
            #[cfg(unix)]
            SockAddr::UDS(_) => None,
        };

        let trusted = match &peer_ip {
            Some(ip) => self.cfg.trusted_proxies.contains(ip),
            None => true,
        };
        if !trusted {
            return peer_ip;
        }

        match headers.get(&self.cfg.client_ip_header) {
            Some(_) => {
                let chain = self.forwarded_chain(headers);
                match chain {
                    Some(addrs) if !addrs.is_empty() => Some(select_client_ip(&addrs, &self.cfg.trusted_proxies)),
                    _ => peer_ip,
                }
            }
            None => peer_ip,
        }
    }

    fn forwarded_chain(&self, headers: &HeaderMap) -> Option<Vec<IpAddr>> {
        let mut addrs = Vec::new();
        for value in &headers.get_all(&self.cfg.client_ip_header) {
            let value = value.to_str().ok()?;
            for item in value.split(',') {
                let item = item.trim();
                if item.is_empty() {
                    continue;
                }
                let item = item.strip_prefix('[').and_then(|v| v.strip_suffix(']')).unwrap_or(item);
                addrs.push(normalize_ip(item.parse::<IpAddr>().ok()?));
            }
        }
        Some(addrs)
    }

    fn outer_key(&self, ip: IpAddr) -> OuterKey {
        match ip {
            IpAddr::V4(v4) => {
                let bits = u32::from(v4);
                let prefix = self.cfg.outer_prefix_v4.min(32);
                let mask = mask_u32(prefix);
                OuterKey {
                    family: 4,
                    masked: u128::from(bits & mask),
                }
            }
            IpAddr::V6(v6) => {
                let bits = u128::from(v6);
                let prefix = self.cfg.outer_prefix_v6.min(128);
                let mask = mask_u128(prefix);
                OuterKey {
                    family: 6,
                    masked: bits & mask,
                }
            }
        }
    }

    fn inner_key(&self, hasher: &impl BuildHasher, headers: &HeaderMap, ip: IpAddr) -> u64 {
        let mut h = hasher.build_hasher();
        let write_attr = |h: &mut dyn std::hash::Hasher, name: &str, values: &[&str]| {
            h.write(name.as_bytes());
            h.write_u8(0);
            if values.is_empty() {
                h.write_u8(0);
            } else {
                for value in values {
                    h.write(value.as_bytes());
                    h.write_u8(0);
                }
            }
        };

        if self.cfg.inner_include_ip {
            let ip = ip.to_string();
            write_attr(&mut h, "client-ip", &[ip.as_str()]);
        }
        for name in &self.cfg.inner_headers {
            let values: Vec<&str> = headers
                .get_all(name.as_str())
                .iter()
                .filter_map(|v| v.to_str().ok())
                .collect();
            write_attr(&mut h, name, &values);
        }
        let cookies = collect_cookies(headers);
        for name in &self.cfg.inner_cookies {
            let value = cookies.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str());
            let values: Vec<&str> = value.into_iter().collect();
            write_attr(&mut h, &format!("cookie:{name}"), &values);
        }

        h.finish()
    }

    fn resolve(&mut self, hasher: &impl BuildHasher, peer: &SockAddr, headers: &HeaderMap) -> Option<FlowRef> {
        let ip = self.client_ip(peer, headers)?;
        let base_outer = self.outer_key(ip);

        let outer = if self.outers.contains_key(&base_outer) || self.outers.len() < self.cfg.outer_max {
            base_outer
        } else {
            match self.overflow_outer {
                Some(key) if self.outers.contains_key(&key) => key,
                _ => {
                    self.overflow_outer = Some(OVERFLOW_OUTER);
                    OVERFLOW_OUTER
                }
            }
        };

        let v = self.v;
        let burst = self.cfg.burst;
        let ikey = self.inner_key(hasher, headers, ip);
        let o = self.outers.entry(outer).or_insert_with(|| OuterFlow::new(v, burst));
        o.in_flight += 1;
        o.last_active = Instant::now();

        let inner = if o.inners.contains_key(&ikey)
            || (o.inners.len() < self.cfg.inner_max_per_outer && self.inner_count < self.cfg.inner_max)
        {
            ikey
        } else {
            INNER_OVERFLOW
        };

        let ov = o.v;
        let inner_ref = o.inners.entry(inner).or_insert_with(|| {
            self.inner_count += 1;
            InnerFlow::new(ov, burst)
        });
        inner_ref.in_flight += 1;
        inner_ref.last_active = Instant::now();

        Some(FlowRef { outer, inner })
    }
}

/// Per-worker two-level fair queue. One instance is shared by every runtime of a worker.
pub(crate) struct FairScheduler {
    q: Mutex<FairQueue>,
    cv: Condvar,
    hasher: std::collections::hash_map::RandomState,
}

impl FairScheduler {
    pub fn new(cfg: FairConfig) -> Self {
        Self {
            q: Mutex::new(FairQueue::new(cfg.inner)),
            cv: Condvar::new(),
            hasher: std::collections::hash_map::RandomState::new(),
        }
    }

    pub fn resolve(&self, peer: &SockAddr, headers: &HeaderMap) -> Option<FlowRef> {
        self.q.lock().unwrap().resolve(&self.hasher, peer, headers)
    }

    pub fn release(&self, flow: FlowRef) {
        self.q.lock().unwrap().release(flow);
    }

    pub fn debit(&self, flow: &FlowRef, cost_ns: u64) {
        self.q.lock().unwrap().debit(flow, cost_ns);
    }

    pub fn push(&self, task: BlockingTask, flow: Option<&FlowRef>) -> Result<(), PushError> {
        let mut q = self.q.lock().unwrap();
        if q.closed {
            return Err(PushError);
        }
        q.push(task, flow.copied());
        drop(q);
        self.cv.notify_one();
        Ok(())
    }

    pub fn pop(&self) -> Option<BlockingTask> {
        let mut q = self.q.lock().unwrap();
        loop {
            if let Some(task) = q.pop_task() {
                return Some(task);
            }
            if q.closed {
                return None;
            }
            q.gc();
            if let Some(task) = q.pop_task() {
                return Some(task);
            }
            if q.closed {
                return None;
            }
            q = self.cv.wait(q).unwrap();
        }
    }

    pub fn pop_timeout(&self, d: Duration) -> Result<BlockingTask, PopError> {
        let mut q = self.q.lock().unwrap();
        let deadline = Instant::now() + d;
        loop {
            if let Some(task) = q.pop_task() {
                return Ok(task);
            }
            if q.closed {
                return Err(PopError::Closed);
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(PopError::Timeout);
            }
            q.gc();
            if let Some(task) = q.pop_task() {
                return Ok(task);
            }
            let (guard, _) = self.cv.wait_timeout(q, deadline - now).unwrap();
            q = guard;
        }
    }

    pub fn len(&self) -> usize {
        self.q.lock().unwrap().queued
    }

    pub fn lane_enqueue(&self, flow: &FlowRef, fut: Py<PyAny>) -> bool {
        self.q.lock().unwrap().lane_enqueue(*flow, fut)
    }

    pub fn lane_next(&self) -> Option<Py<PyAny>> {
        self.q.lock().unwrap().lane_next()
    }

    pub fn lane_release(&self) -> bool {
        self.q.lock().unwrap().lane_release()
    }
}

impl Drop for FairScheduler {
    fn drop(&mut self) {
        if let Ok(mut q) = self.q.lock() {
            q.closed = true;
        }
        self.cv.notify_all();
    }
}

impl Scheduler for FairScheduler {
    fn resolve(&self, peer: &SockAddr, headers: &HeaderMap) -> Option<FlowRef> {
        FairScheduler::resolve(self, peer, headers)
    }
    fn release(&self, flow: FlowRef) {
        FairScheduler::release(self, flow);
    }
    fn debit(&self, flow: &FlowRef, cost_ns: u64) {
        FairScheduler::debit(self, flow, cost_ns);
    }
    fn push(&self, task: BlockingTask, flow: Option<&FlowRef>) -> Result<(), PushError> {
        FairScheduler::push(self, task, flow)
    }
    fn pop(&self) -> Option<BlockingTask> {
        FairScheduler::pop(self)
    }
    fn pop_timeout(&self, d: Duration) -> Result<BlockingTask, PopError> {
        FairScheduler::pop_timeout(self, d)
    }
    fn len(&self) -> usize {
        FairScheduler::len(self)
    }
    fn lane_enqueue(&self, flow: &FlowRef, fut: Py<PyAny>) -> bool {
        FairScheduler::lane_enqueue(self, flow, fut)
    }
    fn lane_next(&self) -> Option<Py<PyAny>> {
        FairScheduler::lane_next(self)
    }
    fn lane_release(&self) -> bool {
        FairScheduler::lane_release(self)
    }
}

// ---------------------------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------------------------

/// Dispatch over the two concrete schedulers: one predictable branch per call, no vtable,
/// so that `RuntimeRef` can stay non-generic.
// The fair variant is much larger than the FIFO one; the extra indirection of a Box is
// not worth it here, as the enum is stored behind a single `Arc` per worker.
#[allow(clippy::large_enum_variant)]
pub(crate) enum SchedulerImpl {
    Fifo(FifoScheduler),
    Fair(FairScheduler),
}

impl SchedulerImpl {
    pub fn fifo() -> Self {
        Self::Fifo(FifoScheduler::new())
    }

    pub fn fair(cfg: FairConfig) -> Self {
        Self::Fair(FairScheduler::new(cfg))
    }
}

impl Scheduler for SchedulerImpl {
    #[inline]
    fn resolve(&self, peer: &SockAddr, headers: &HeaderMap) -> Option<FlowRef> {
        match self {
            Self::Fifo(s) => s.resolve(peer, headers),
            Self::Fair(s) => s.resolve(peer, headers),
        }
    }

    #[inline]
    fn release(&self, flow: FlowRef) {
        match self {
            Self::Fifo(s) => s.release(flow),
            Self::Fair(s) => s.release(flow),
        }
    }

    #[inline]
    fn debit(&self, flow: &FlowRef, cost_ns: u64) {
        match self {
            Self::Fifo(s) => s.debit(flow, cost_ns),
            Self::Fair(s) => s.debit(flow, cost_ns),
        }
    }

    #[inline]
    fn push(&self, task: BlockingTask, flow: Option<&FlowRef>) -> Result<(), PushError> {
        match self {
            Self::Fifo(s) => s.push(task, flow),
            Self::Fair(s) => s.push(task, flow),
        }
    }

    #[inline]
    fn pop(&self) -> Option<BlockingTask> {
        match self {
            Self::Fifo(s) => s.pop(),
            Self::Fair(s) => s.pop(),
        }
    }

    #[inline]
    fn pop_timeout(&self, d: Duration) -> Result<BlockingTask, PopError> {
        match self {
            Self::Fifo(s) => s.pop_timeout(d),
            Self::Fair(s) => s.pop_timeout(d),
        }
    }

    #[inline]
    fn len(&self) -> usize {
        match self {
            Self::Fifo(s) => s.len(),
            Self::Fair(s) => s.len(),
        }
    }

    #[inline]
    fn lane_enqueue(&self, flow: &FlowRef, fut: Py<PyAny>) -> bool {
        match self {
            Self::Fifo(s) => s.lane_enqueue(flow, fut),
            Self::Fair(s) => s.lane_enqueue(flow, fut),
        }
    }

    #[inline]
    fn lane_next(&self) -> Option<Py<PyAny>> {
        match self {
            Self::Fifo(s) => s.lane_next(),
            Self::Fair(s) => s.lane_next(),
        }
    }

    #[inline]
    fn lane_release(&self) -> bool {
        match self {
            Self::Fifo(s) => s.lane_release(),
            Self::Fair(s) => s.lane_release(),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------------------------

/// Fair-scheduler knobs, mirroring the `--fq-*` options and `GRANIAN_FQ_*` env vars.
#[pyclass(frozen, name = "FairConfig", module = "granian._granian")]
#[derive(Clone)]
pub(crate) struct FairConfig {
    inner: FairConfigInner,
    #[pyo3(get)]
    pub(crate) monitoring_tool_id: u8,
}

#[pymethods]
impl FairConfig {
    #[new]
    #[pyo3(signature = (
        outer_prefix_v4=24,
        outer_prefix_v6=56,
        outer_max=65_536,
        inner_max_per_outer=1_024,
        inner_max=262_144,
        burst_ms=20,
        max_debt_ms=5_000,
        debt_ttl_secs=30,
        trusted_proxies=vec!["127.0.0.1".to_string(), "::1".to_string()],
        client_ip_header="x-forwarded-for".to_string(),
        inner_headers=vec![
            "user-agent".to_string(),
            "accept-language".to_string(),
            "sec-ch-ua".to_string(),
            "sec-ch-ua-platform".to_string(),
            "sec-ch-ua-mobile".to_string(),
            "authorization".to_string(),
        ],
        inner_cookies=vec!["sessionid".to_string(), "session".to_string()],
        inner_include_ip=true,
        monitoring_tool_id=4,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        outer_prefix_v4: u8,
        outer_prefix_v6: u8,
        outer_max: usize,
        inner_max_per_outer: usize,
        inner_max: usize,
        burst_ms: u64,
        max_debt_ms: u64,
        debt_ttl_secs: u64,
        trusted_proxies: Vec<String>,
        client_ip_header: String,
        inner_headers: Vec<String>,
        inner_cookies: Vec<String>,
        inner_include_ip: bool,
        monitoring_tool_id: u8,
    ) -> PyResult<Self> {
        let client_ip_header = HeaderName::from_bytes(client_ip_header.as_bytes())
            .map_err(|_| pyo3::exceptions::PyValueError::new_err("invalid client IP header name"))?;

        Ok(Self {
            inner: FairConfigInner {
                outer_prefix_v4,
                outer_prefix_v6,
                outer_max: outer_max.max(1),
                inner_max_per_outer: inner_max_per_outer.max(1),
                inner_max: inner_max.max(1),
                burst: burst_ms.saturating_mul(1_000_000),
                max_debt: max_debt_ms.saturating_mul(1_000_000),
                debt_ttl: Duration::from_secs(debt_ttl_secs.max(1)),
                trusted_proxies: Arc::new(TrustedProxies::parse(&trusted_proxies)),
                client_ip_header,
                inner_headers,
                inner_cookies,
                inner_include_ip,
            },
            monitoring_tool_id,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

/// Peers whose client-IP header may be trusted, as a list of IPs and CIDR networks
/// (or `*` for "trust everything").
pub(crate) struct TrustedProxies {
    always: bool,
    hosts: Vec<IpAddr>,
    nets: Vec<(IpAddr, u8)>,
}

impl TrustedProxies {
    pub fn parse(values: &[String]) -> Self {
        let always = values.iter().any(|v| v == "*");
        if always {
            return Self {
                always,
                hosts: Vec::new(),
                nets: Vec::new(),
            };
        }

        let mut hosts = Vec::new();
        let mut nets = Vec::new();
        for value in values {
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
            match value.split_once('/') {
                Some((host, prefix)) => {
                    if let (Ok(ip), Ok(prefix)) = (host.trim().parse::<IpAddr>(), prefix.trim().parse::<u8>()) {
                        nets.push((normalize_ip(ip), prefix));
                    }
                }
                None => {
                    if let Ok(ip) = value.parse::<IpAddr>() {
                        hosts.push(normalize_ip(ip));
                    }
                }
            }
        }

        Self { always, hosts, nets }
    }

    pub fn contains(&self, ip: &IpAddr) -> bool {
        if self.always {
            return true;
        }
        if self.hosts.contains(ip) {
            return true;
        }
        self.nets.iter().any(|(net, prefix)| match (net, ip) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => {
                let prefix = (*prefix).min(32);
                u32::from(*ip) & mask_u32(prefix) == u32::from(*net) & mask_u32(prefix)
            }
            (IpAddr::V6(net), IpAddr::V6(ip)) => {
                let prefix = (*prefix).min(128);
                u128::from(*ip) & mask_u128(prefix) == u128::from(*net) & mask_u128(prefix)
            }
            _ => false,
        })
    }
}

fn select_client_ip(chain: &[IpAddr], trusted: &TrustedProxies) -> IpAddr {
    if trusted.always {
        return chain[0];
    }
    for ip in chain.iter().rev() {
        if !trusted.contains(ip) {
            return *ip;
        }
    }
    chain[0]
}

fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(IpAddr::V6(v6), IpAddr::V4),
        IpAddr::V4(_) => ip,
    }
}

fn mask_u32(prefix: u8) -> u32 {
    if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) }
}

fn mask_u128(prefix: u8) -> u128 {
    if prefix == 0 { 0 } else { u128::MAX << (128 - prefix) }
}

fn collect_cookies(headers: &HeaderMap) -> Vec<(String, String)> {
    let mut cookies = Vec::new();
    for value in &headers.get_all(hyper::header::COOKIE) {
        let Ok(value) = value.to_str() else {
            continue;
        };
        for pair in value.split(';') {
            let pair = pair.trim();
            if let Some((name, value)) = pair.split_once('=') {
                cookies.push((name.trim().to_string(), value.trim().to_string()));
            }
        }
    }
    cookies
}

/// Convenience constructors for tests / internal use.
#[allow(dead_code)]
impl OuterKey {
    pub fn from_v4(addr: Ipv4Addr) -> Self {
        Self {
            family: 4,
            masked: u128::from(u32::from(addr)),
        }
    }

    pub fn from_v6(addr: Ipv6Addr) -> Self {
        Self {
            family: 6,
            masked: u128::from(addr),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::header::HeaderValue;
    use std::net::SocketAddr;

    fn cfg() -> FairConfig {
        FairConfig::new(
            24,
            56,
            65_536,
            1_024,
            262_144,
            20,
            5_000,
            30,
            vec!["127.0.0.1".to_string(), "::1".to_string()],
            "x-forwarded-for".to_string(),
            vec!["user-agent".to_string()],
            vec!["sessionid".to_string()],
            true,
            4,
        )
        .unwrap()
    }

    fn peer(v: [u8; 4]) -> SockAddr {
        SockAddr::TCP(SocketAddr::from((v, 1234)))
    }

    fn task(order: &Arc<Mutex<Vec<u32>>>, id: u32) -> BlockingTask {
        let order = order.clone();
        BlockingTask::new(move |_py| order.lock().unwrap().push(id))
    }

    fn run_task(task: BlockingTask) {
        pyo3::Python::initialize();
        pyo3::Python::attach(move |py| task.run(py));
    }

    #[test]
    fn budget_orders_backlogged_flows() {
        let sched = FairScheduler::new(cfg());
        let order = Arc::new(Mutex::new(Vec::new()));

        let flow_a = sched.resolve(&peer([10, 0, 0, 1]), &HeaderMap::new()).unwrap();
        let flow_b = sched.resolve(&peer([10, 1, 0, 1]), &HeaderMap::new()).unwrap();
        assert_ne!(flow_a.outer, flow_b.outer);

        sched.push(task(&order, 0), Some(&flow_a)).unwrap();
        sched.push(task(&order, 1), Some(&flow_b)).unwrap();
        assert_eq!(sched.len(), 2);

        // A burned a lot of lane time, so B must be served first.
        sched.debit(&flow_a, 100_000_000);

        let first = sched.pop_timeout(Duration::from_millis(0)).unwrap();
        run_task(first);
        let second = sched.pop_timeout(Duration::from_millis(0)).unwrap();
        run_task(second);

        assert_eq!(*order.lock().unwrap(), vec![1, 0]);
    }

    #[test]
    fn system_lane_is_served_first() {
        let sched = FairScheduler::new(cfg());
        let order = Arc::new(Mutex::new(Vec::new()));

        let flow = sched.resolve(&peer([10, 0, 0, 1]), &HeaderMap::new()).unwrap();
        sched.push(task(&order, 0), Some(&flow)).unwrap();
        sched.push(task(&order, 1), None).unwrap();

        run_task(sched.pop_timeout(Duration::from_millis(0)).unwrap());
        run_task(sched.pop_timeout(Duration::from_millis(0)).unwrap());

        assert_eq!(*order.lock().unwrap(), vec![1, 0]);
    }

    #[test]
    fn events_within_a_flow_stay_ordered() {
        let sched = FairScheduler::new(cfg());
        let order = Arc::new(Mutex::new(Vec::new()));

        let flow = sched.resolve(&peer([10, 0, 0, 1]), &HeaderMap::new()).unwrap();
        for id in 0..5 {
            sched.push(task(&order, id), Some(&flow)).unwrap();
        }
        for _ in 0..5 {
            let task = sched.pop_timeout(Duration::from_millis(0)).unwrap();
            run_task(task);
        }

        assert_eq!(*order.lock().unwrap(), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn outer_table_overflow_shares_one_flow() {
        let mut config = cfg();
        config.inner.outer_max = 1;
        let sched = FairScheduler::new(config);

        let a = sched.resolve(&peer([10, 0, 0, 1]), &HeaderMap::new()).unwrap();
        let b = sched.resolve(&peer([10, 1, 0, 1]), &HeaderMap::new()).unwrap();
        let c = sched.resolve(&peer([10, 2, 0, 1]), &HeaderMap::new()).unwrap();

        assert_ne!(a.outer, b.outer);
        // once the table is full, every new outer prefix lands in the shared overflow flow
        assert_eq!(b.outer, c.outer);
        assert_eq!(b.outer, OVERFLOW_OUTER);
    }

    #[test]
    fn release_evicts_idle_flows() {
        let sched = FairScheduler::new(cfg());
        let flow = sched.resolve(&peer([10, 0, 0, 1]), &HeaderMap::new()).unwrap();
        assert_eq!(sched.q.lock().unwrap().outers.len(), 1);
        sched.release(flow);
        let q = sched.q.lock().unwrap();
        assert!(q.outers.is_empty());
        assert_eq!(q.inner_count, 0);
    }

    #[test]
    fn client_ip_comes_from_the_forwarded_header() {
        let sched = FairScheduler::new(cfg());

        let mut headers = HeaderMap::new();
        headers.append("x-forwarded-for", HeaderValue::from_static("203.0.113.5, 127.0.0.1"));

        // behind a trusted proxy, the header decides the outer prefix
        let proxied = sched.resolve(&peer([127, 0, 0, 1]), &headers).unwrap();
        let direct = sched.resolve(&peer([203, 0, 113, 5]), &HeaderMap::new()).unwrap();
        assert_eq!(proxied.outer, direct.outer);
        assert_eq!(proxied.outer, OuterKey::from_v4(Ipv4Addr::new(203, 0, 113, 0)));

        // an untrusted peer is taken as-is, header ignored
        let untrusted = sched.resolve(&peer([198, 51, 100, 7]), &headers).unwrap();
        assert_eq!(untrusted.outer, OuterKey::from_v4(Ipv4Addr::new(198, 51, 100, 0)));
    }

    #[test]
    fn inner_key_tracks_the_fingerprint() {
        let sched = FairScheduler::new(cfg());
        let mut headers = HeaderMap::new();
        headers.append("user-agent", HeaderValue::from_static("browser-a"));

        let first = sched.resolve(&peer([10, 0, 0, 1]), &headers).unwrap();
        let same = sched.resolve(&peer([10, 0, 0, 1]), &headers).unwrap();
        headers.insert("user-agent", HeaderValue::from_static("browser-b"));
        let other = sched.resolve(&peer([10, 0, 0, 1]), &headers).unwrap();

        assert_eq!(first.inner, same.inner);
        assert_ne!(first.inner, other.inner);
        assert_eq!(first.outer, other.outer);
    }

    #[test]
    fn ipv4_mapped_ipv6_is_normalized() {
        let sched = FairScheduler::new(cfg());
        let mapped = sched
            .resolve(&peer_v6("::ffff:203.0.113.9"), &HeaderMap::new())
            .unwrap();
        assert_eq!(mapped.outer, OuterKey::from_v4(Ipv4Addr::new(203, 0, 113, 0)));
    }

    /// Queue a turn identified by `id` (a Python int, so the granted turn can be told apart).
    fn turn(sched: &FairScheduler, flow: &FlowRef, id: u32) -> bool {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            let fut = id.into_pyobject(py).unwrap().into_any().unbind();
            sched.lane_enqueue(flow, fut)
        })
    }

    fn next_turn(sched: &FairScheduler) -> Option<u32> {
        pyo3::Python::attach(|py| sched.lane_next().map(|fut| fut.extract::<u32>(py).unwrap()))
    }

    #[test]
    fn loop_lane_dispatch_protocol() {
        let sched = FairScheduler::new(cfg());
        let flow = sched.resolve(&peer([10, 0, 0, 1]), &HeaderMap::new()).unwrap();

        // the first turn on an idle lane asks for a dispatch, later ones do not
        assert!(turn(&sched, &flow, 0));
        assert!(!turn(&sched, &flow, 1));

        assert_eq!(next_turn(&sched), Some(0));
        // the granted step ends while a turn is waiting: dispatch again
        assert!(sched.lane_release());
        assert_eq!(next_turn(&sched), Some(1));
        // nobody is waiting anymore: the lane goes idle
        assert!(!sched.lane_release());
        assert!(turn(&sched, &flow, 2));
        assert_eq!(next_turn(&sched), Some(2));
        assert!(!sched.lane_release());
        // a dispatch that finds nothing (e.g. only cancelled turns) also idles the lane
        assert!(turn(&sched, &flow, 3));
        assert_eq!(next_turn(&sched), Some(3));
        assert_eq!(next_turn(&sched), None);
        assert!(turn(&sched, &flow, 4));
    }

    #[test]
    fn loop_lane_orders_steps_by_budget() {
        let sched = FairScheduler::new(cfg());
        let hog = sched
            .resolve(
                &peer([10, 0, 0, 1]),
                &HeaderMap::from_iter([(hyper::header::USER_AGENT, HeaderValue::from_static("hog"))]),
            )
            .unwrap();
        let light = sched
            .resolve(
                &peer([10, 0, 0, 1]),
                &HeaderMap::from_iter([(hyper::header::USER_AGENT, HeaderValue::from_static("light"))]),
            )
            .unwrap();
        assert_eq!(hog.outer, light.outer);
        assert_ne!(hog.inner, light.inner);

        // the hog has several steps ready, then the light flow becomes ready too
        turn(&sched, &hog, 0);
        turn(&sched, &hog, 1);
        turn(&sched, &hog, 2);
        turn(&sched, &light, 10);

        // the hog's previous steps burned 50ms of loop time
        sched.debit(&hog, 50_000_000);

        assert_eq!(next_turn(&sched), Some(10));
        assert!(sched.lane_release());
        // the hog's own steps keep their order
        assert_eq!(next_turn(&sched), Some(0));
        assert_eq!(next_turn(&sched), Some(1));
        assert_eq!(next_turn(&sched), Some(2));
        assert!(!sched.lane_release());
    }

    #[test]
    fn debits_keep_both_lanes_consistent() {
        let sched = FairScheduler::new(cfg());
        let order = Arc::new(Mutex::new(Vec::new()));
        let a = sched.resolve(&peer([10, 0, 0, 1]), &HeaderMap::new()).unwrap();
        let b = sched.resolve(&peer([10, 1, 0, 1]), &HeaderMap::new()).unwrap();

        // `a` is backlogged on both lanes when it gets debited
        sched.push(task(&order, 0), Some(&a)).unwrap();
        turn(&sched, &a, 0);
        sched.push(task(&order, 1), Some(&b)).unwrap();
        turn(&sched, &b, 1);
        sched.debit(&a, 100_000_000);

        run_task(sched.pop_timeout(Duration::from_millis(0)).unwrap());
        run_task(sched.pop_timeout(Duration::from_millis(0)).unwrap());
        assert_eq!(*order.lock().unwrap(), vec![1, 0]);
        assert_eq!(next_turn(&sched), Some(1));
        assert_eq!(next_turn(&sched), Some(0));
        assert_eq!(next_turn(&sched), None);

        // and the flows can still be released and collected
        sched.release(a);
        sched.release(b);
        assert_eq!(sched.len(), 0);
    }

    fn peer_v6(addr: &str) -> SockAddr {
        let ip: std::net::Ipv6Addr = addr.parse().unwrap();
        SockAddr::TCP(SocketAddr::from((ip, 1234)))
    }
}
