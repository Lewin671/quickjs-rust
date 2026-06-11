//! Microtask (promise job) driving for the bytecode VM.
//!
//! Promise reactions enqueue jobs into the realm-owned queue rather than
//! running synchronously. The top-level [`eval_bytecode`](super::eval_bytecode)
//! path drains the queue before returning; callers that need to control drain
//! timing (the async Test262 harness, the CLI) use [`eval_bytecode_keep_jobs`]
//! and drain explicitly with [`run_pending_jobs`].

use std::collections::HashMap;

use crate::{RuntimeError, Value, promise};

use super::ir::Bytecode;
use super::vm::Vm;
use crate::CallEnv;

/// Runs the script body without draining the promise job queue, returning the
/// completion value alongside the realm environment that still owns any
/// enqueued microtasks. Callers decide when to drain via [`run_pending_jobs`].
pub(super) fn eval_bytecode_keep_jobs(
    bytecode: &Bytecode,
) -> Result<(Value, CallEnv), RuntimeError> {
    let mut vm = Vm::new(bytecode);
    let value = vm.run()?;
    Ok((value, vm.current_env()))
}

/// Drains the realm's pending promise job queue, running queued microtasks in
/// FIFO order until the queue is empty (including jobs that enqueue further
/// jobs). The job queue is owned by the realm environment, so the same `env`
/// returned by [`eval_bytecode_keep_jobs`] must be threaded through here.
pub(super) fn run_pending_jobs(env: &mut CallEnv) -> Result<(), RuntimeError> {
    promise::drain_promise_jobs(env)
}
