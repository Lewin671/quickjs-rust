//! Per-agent execution context for the Test262 `$262.agent` multi-agent
//! harness. The whole module is gated behind the `agents` cargo feature; the
//! default single-threaded build never compiles it.
//!
//! In Phase 1 the context carries only `can_block` — the `AgentCanSuspend()`
//! value that decides whether `Atomics.wait` may suspend the calling agent
//! (`CanBlockIsFalse`-flagged cases set it to `false`, so `Atomics.wait` throws
//! a `TypeError`). Later phases grow this into the full agent-cluster handle
//! (broadcast channels, report queue, shared-buffer waiter registry).

use std::rc::Rc;

/// Execution context for the agent currently running on this OS thread.
pub(crate) struct AgentContext {
    /// `AgentCanSuspend()`. `false` makes `Atomics.wait` throw a `TypeError`
    /// instead of blocking; the ordinary main agent has it `true`.
    pub(crate) can_block: bool,
}

impl AgentContext {
    /// Builds a context for an agent whose `[[CanBlock]]` is `can_block`.
    pub(crate) fn new(can_block: bool) -> AgentContextRef {
        Rc::new(Self { can_block })
    }
}

/// A shared, cheaply cloned handle to the per-thread agent context. It is `Rc`
/// (thread-local) because each agent runs its own engine on its own thread; the
/// cross-thread cluster state added in later phases lives behind `Arc` inside.
pub(crate) type AgentContextRef = Rc<AgentContext>;
