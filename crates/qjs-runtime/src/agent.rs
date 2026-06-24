//! Per-agent execution context and agent-cluster plumbing for the Test262
//! `$262.agent` multi-agent harness. The whole module is gated behind the
//! `agents` cargo feature; the default single-threaded build never compiles it.
//!
//! Each agent is a separate engine running on its own OS thread over its own
//! source string — engine values (`Rc`-based) never cross threads. The only
//! things that cross a thread boundary are the `Arc`-shared agent cluster
//! (report queue + broadcast channels) and the `Arc`-shared `SharedArrayBuffer`
//! backing delivered over a broadcast channel. A worker reconstructs a local
//! `SharedArrayBuffer` object around the received backing, so all agents read
//! and write one memory region.
//!
//! Native primitives provided here (wired to globals in `global.rs` and
//! reachable from JS as `$262.agent.*`): `start`, `broadcast`, `getReport`,
//! `report`, `sleep`, `monotonicNow`, `receiveBroadcast`, `leaving`. The richer
//! helpers (`safeBroadcast`, `waitUntil`, `tryYield`, `timeouts`, ...) are pure
//! JS supplied by `harness/atomicsHelper.js`.

use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::array_buffer::SharedBackingRef;
use crate::{CallEnv, RuntimeError, Value, call_function, to_js_string_with_env};

/// A `SharedArrayBuffer` handed from the main agent to a worker: the shared
/// backing plus the resizable maximum (so the worker rebuilds a growable buffer
/// faithfully). Cloneable so one broadcast reaches every registered worker.
#[derive(Clone)]
pub(crate) struct BroadcastMessage {
    pub(crate) backing: SharedBackingRef,
    pub(crate) max_byte_length: Option<usize>,
}

/// Cluster state shared by `Arc` across every agent (main + workers): the
/// report queue the main agent drains and the broadcast senders, one per
/// worker. A monotonic baseline backs `$262.agent.monotonicNow`.
pub(crate) struct AgentCluster {
    reports: Mutex<VecDeque<String>>,
    senders: Mutex<Vec<Sender<BroadcastMessage>>>,
    start: Instant,
}

/// A cheaply cloned handle to the shared [`AgentCluster`]. `Send + Sync`, so it
/// rides into spawned worker threads.
pub(crate) type AgentClusterRef = Arc<AgentCluster>;

/// Locks `mutex`, recovering from a poisoned lock (a worker may panic while
/// holding it; the queued data stays valid).
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}

impl AgentCluster {
    fn new() -> AgentClusterRef {
        Arc::new(Self {
            reports: Mutex::new(VecDeque::new()),
            senders: Mutex::new(Vec::new()),
            start: Instant::now(),
        })
    }

    /// Appends a worker's report; read FIFO by the main agent's `getReport`.
    fn push_report(&self, report: String) {
        lock(&self.reports).push_back(report);
    }

    /// Removes and returns the oldest report, or `None` when the queue is empty.
    fn pop_report(&self) -> Option<String> {
        lock(&self.reports).pop_front()
    }

    /// Registers a fresh broadcast channel for a new worker, returning its
    /// receiver. The main agent keeps the sender so a later `broadcast` reaches
    /// this worker.
    fn register_worker(&self) -> Receiver<BroadcastMessage> {
        let (sender, receiver) = channel();
        lock(&self.senders).push(sender);
        receiver
    }

    /// Sends `message` to every registered worker. A worker that has already
    /// finished (its receiver dropped) is silently skipped.
    fn broadcast(&self, message: &BroadcastMessage) {
        for sender in lock(&self.senders).iter() {
            let _ = sender.send(message.clone());
        }
    }

    /// Milliseconds elapsed since the cluster (process) started.
    fn monotonic_now_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}

/// The execution context for the agent running on this OS thread.
pub(crate) struct AgentContext {
    /// `AgentCanSuspend()`. `false` makes `Atomics.wait` throw a `TypeError`.
    pub(crate) can_block: bool,
    /// Cluster shared with every other agent.
    cluster: AgentClusterRef,
    /// A worker's broadcast receiver, taken by its first `receiveBroadcast`
    /// call. `None` for the main agent.
    inbox: Mutex<Option<Receiver<BroadcastMessage>>>,
}

/// A shared, cheaply cloned handle to the per-thread agent context.
pub(crate) type AgentContextRef = Rc<AgentContext>;

impl AgentContext {
    /// Builds the main agent's context (it owns a fresh cluster). `can_block`
    /// is `AgentCanSuspend()` — `false` for `CanBlockIsFalse` cases.
    pub(crate) fn main(can_block: bool) -> AgentContextRef {
        Rc::new(Self {
            can_block,
            cluster: AgentCluster::new(),
            inbox: Mutex::new(None),
        })
    }

    /// Builds a worker agent's context, sharing the main agent's cluster and
    /// owning the broadcast receiver registered for this worker.
    fn worker(cluster: AgentClusterRef, inbox: Receiver<BroadcastMessage>) -> AgentContextRef {
        Rc::new(Self {
            can_block: true,
            cluster,
            inbox: Mutex::new(Some(inbox)),
        })
    }
}

/// Reads the current agent context, erroring if `$262.agent` is used outside an
/// agent (i.e. the engine was not entered through the harness).
fn agent_context(env: &CallEnv) -> Result<AgentContextRef, RuntimeError> {
    env.agent_context().ok_or_else(|| RuntimeError {
        thrown: None,
        message: "TypeError: $262.agent is not available in this context".to_owned(),
    })
}

/// `$262.agent.start(source)`: spawns a worker agent on a new OS thread running
/// `source`, sharing this agent's cluster.
pub(crate) fn native_agent_start(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let context = agent_context(env)?;
    let source = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let cluster = Arc::clone(&context.cluster);
    let inbox = cluster.register_worker();
    // The worker builds its own engine, values, and SharedArrayBuffer objects on
    // its thread; only the `Send` cluster, receiver, and source string cross.
    std::thread::spawn(move || run_worker(&source, cluster, inbox));
    Ok(Value::Undefined)
}

/// `$262.agent.broadcast(sab)`: delivers `sab`'s shared backing to every worker.
pub(crate) fn native_agent_broadcast(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let context = agent_context(env)?;
    let Some(Value::Object(object)) = argument_values.first() else {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: $262.agent.broadcast requires a SharedArrayBuffer".to_owned(),
        });
    };
    let Some((backing, max_byte_length)) =
        crate::array_buffer::shared_array_buffer_backing_parts(object)
    else {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: $262.agent.broadcast requires a SharedArrayBuffer".to_owned(),
        });
    };
    context.cluster.broadcast(&BroadcastMessage {
        backing,
        max_byte_length,
    });
    Ok(Value::Undefined)
}

/// `$262.agent.getReport()`: pops the oldest worker report, or `null`.
pub(crate) fn native_agent_get_report(env: &mut CallEnv) -> Result<Value, RuntimeError> {
    let context = agent_context(env)?;
    Ok(match context.cluster.pop_report() {
        Some(report) => Value::String(report.into()),
        None => Value::Null,
    })
}

/// `$262.agent.report(value)`: appends a report string for the main agent.
pub(crate) fn native_agent_report(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let context = agent_context(env)?;
    let report = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    context.cluster.push_report(report.to_string());
    Ok(Value::Undefined)
}

/// `$262.agent.sleep(ms)`: blocks this agent's thread for `ms` milliseconds.
pub(crate) fn native_agent_sleep(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let ms = crate::to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if ms.is_finite() && ms > 0.0 {
        std::thread::sleep(Duration::from_millis(ms as u64));
    }
    Ok(Value::Undefined)
}

/// `$262.agent.monotonicNow()`: milliseconds since the cluster started.
pub(crate) fn native_agent_monotonic_now(env: &mut CallEnv) -> Result<Value, RuntimeError> {
    let context = agent_context(env)?;
    Ok(Value::Number(context.cluster.monotonic_now_ms()))
}

/// `$262.agent.receiveBroadcast(callback)`: blocks until the main agent
/// broadcasts a buffer, then calls `callback(sab)` with a local
/// `SharedArrayBuffer` wrapping the shared backing.
pub(crate) fn native_agent_receive_broadcast(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let context = agent_context(env)?;
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let receiver = lock(&context.inbox).take().ok_or_else(|| RuntimeError {
        thrown: None,
        message: "TypeError: $262.agent.receiveBroadcast is only available to a worker agent once"
            .to_owned(),
    })?;
    // Block this worker thread until the main agent broadcasts. A dropped sender
    // (main agent finished) ends the worker quietly.
    let Ok(message) = receiver.recv() else {
        return Ok(Value::Undefined);
    };
    let buffer = crate::array_buffer::shared_array_buffer_from_backing(
        env,
        message.backing,
        message.max_byte_length,
    );
    call_function(
        callback,
        Value::Undefined,
        vec![Value::Object(buffer)],
        env,
        false,
    )?;
    Ok(Value::Undefined)
}

/// `$262.agent.leaving()`: signals the worker is done. The thread exits once the
/// worker source returns, so this is a no-op acknowledgement.
pub(crate) fn native_agent_leaving() -> Result<Value, RuntimeError> {
    Ok(Value::Undefined)
}

/// JS prelude prepended to a worker's source so `$262.agent.*` resolves to the
/// native globals installed in every realm. The richer helpers come from
/// `atomicsHelper.js` in tests that need them, but a worker source only uses
/// these base primitives.
const WORKER_PRELUDE: &str = "var $262 = { agent: {\
 start: __quickjsRustAgentStart,\
 broadcast: __quickjsRustAgentBroadcast,\
 getReport: __quickjsRustAgentGetReport,\
 report: __quickjsRustAgentReport,\
 sleep: __quickjsRustAgentSleep,\
 monotonicNow: __quickjsRustAgentMonotonicNow,\
 receiveBroadcast: __quickjsRustAgentReceiveBroadcast,\
 leaving: __quickjsRustAgentLeaving,\
} };\n";

/// Runs a worker agent's source to completion on its own thread. Failures are
/// swallowed: a correct worker reports its result through shared memory before
/// returning, and there is no main-agent channel to surface a Rust error on.
fn run_worker(source: &str, cluster: AgentClusterRef, inbox: Receiver<BroadcastMessage>) {
    let context = AgentContext::worker(cluster, inbox);
    let full_source = format!("{WORKER_PRELUDE}{source}");
    let _ = crate::eval_worker_source(&full_source, context);
}
