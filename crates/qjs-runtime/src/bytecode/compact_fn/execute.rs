//! The compact register executor.
//!
//! This is deliberately a small, separate, `#[inline(never)]` symbol. The
//! whole point of the tier is that its dispatch loop is short enough for the
//! register allocator to keep `pc` and the register base in machine registers,
//! which `Vm::run_current_activation` demonstrably cannot do. Anything that
//! inflates this function -- an extra opcode family, an inlined slow path --
//! spends the budget the tier exists to protect. Keep cold work behind
//! `#[inline(never)]` helpers.
//!
//! The dispatch loop itself lives in `activation::run_frames`, inlined into
//! the frame stack that drives it: handing an action across a function
//! boundary measured worse than the nested Rust call it replaced. What stays
//! here is what both the loop and the frame stack use -- the register store
//! and the cold error constructors.

use crate::{RuntimeError, Value};

/// Why an activation stopped running.
pub(super) enum Action {
    /// The body returned this value.
    Return(Value),
    /// The body reached a call. Registers are frame-relative.
    Call {
        dst: u16,
        base: u16,
        argc: u8,
        /// Where this body resumes once the result is stored in `dst`.
        resume_pc: usize,
    },
}

/// Overwrites a register, deciding inline whether the old value owns anything.
///
/// `drop_in_place::<Value>` stays an out-of-line call and was 22% of the
/// recursive sentinel's profile -- for registers that only ever hold numbers.
/// Testing the discriminant here lets the common case skip the call entirely.
///
/// The four variants listed own nothing; everything else, including any
/// variant added later, takes the ordinary drop. A mistake in that list
/// therefore costs a branch, never a leak.
#[inline(always)]
pub(super) fn store(slot: &mut Value, value: Value) {
    let previous = std::mem::replace(slot, value);
    if matches!(
        previous,
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined
    ) {
        std::mem::forget(previous);
    }
}

#[cold]
pub(super) fn constant_out_of_bounds() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "compact function constant index out of bounds".to_owned(),
    }
}

#[cold]
pub(super) fn uninitialized_local() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "compact function read an uninitialized local slot".to_owned(),
    }
}
