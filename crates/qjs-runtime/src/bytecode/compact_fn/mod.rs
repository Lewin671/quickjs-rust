//! Whole-function compact register execution.
//!
//! Where `typed_loop` accelerates a *loop region* with scalar registers, this
//! tier runs a *whole function body* on `Value` registers, in a small executor
//! of its own that builds no `Vm` and no `FrameState`. That is what a
//! recursive body needs: its cost is spread across activations, not
//! concentrated in a backedge.
//!
//! This module is the numeric tier: constants, local moves, binary operators,
//! jumps, calls and returns (`CompactOp`). `numeric_plan` lowers an admitted
//! body further, to `f64` registers, and `wide` is a second executor for
//! bodies that also touch properties, call methods, or loop. Entry points are
//! `try_run_in_caller_env` and `try_run_standalone` (`activation`).
//!
//! Deliberate boundaries, which are what make the tier safe:
//!
//! - **All-or-nothing admission.** A body is either fully representable or is
//!   not admitted at all. There is no deoptimization after entry, so there is
//!   no replay-after-side-effects problem to solve.
//! - **Admitted callees share the caller's loop.** A call to a body this tier
//!   also admits is a frame push on `activation`'s explicit frame stack, not a
//!   nested `Vm` and not a nested Rust activation. Everything else re-enters
//!   the ordinary call path.
//! - **The operand stack becomes registers.** Stack slots are assigned to
//!   register indices at compile time, which is what removes the push/pop
//!   traffic that dominates a small body.

use qjs_ast::BinaryOp;

use super::ir::Bytecode;

mod activation;
mod compile;
mod creation_cache;
mod execute;
mod numeric_plan;
mod property;
pub(in crate::bytecode) mod wide;
pub(in crate::bytecode) use property::field_initializer_member_read;
pub(in crate::bytecode) use wide::{hands_back_at, resumes_at};

pub(crate) use activation::try_run_in_caller_env;
pub(super) use activation::try_run_standalone;
pub(in crate::bytecode) use numeric_plan::NumericPlan;

/// Bodies wider than this are not worth a register file; the limit also keeps
/// register indices in `u16`.
const MAX_REGISTERS: usize = 256;

/// Register-addressed form of the subset of `Op` this tier admits.
///
/// Every operand is a register index into one flat file: indices below
/// `local_count` are the frame's locals, and the rest are the compile-time
/// assignment of what was the operand stack.
#[derive(Debug, Clone, Copy)]
enum CompactOp {
    LoadConst {
        dst: u16,
        index: u32,
    },
    /// Copies between registers. Locals occupy the low registers, so a local
    /// read or write is this operation rather than a trip through indexed
    /// frame storage.
    Move {
        dst: u16,
        src: u16,
    },
    /// Reads a local backed by a received upvalue cell, which a slot-seeded
    /// direct frame resolves through the function it retains.
    LoadUpvalueLocal {
        dst: u16,
        slot: u16,
    },
    /// Releases a register that `Op::Pop` discarded.
    ///
    /// This is not bookkeeping that could be folded into the compile-time
    /// depth: a discarded register may hold the last reference to an object,
    /// and leaving it live until the activation ends would delay that drop
    /// past the point the source language specifies.
    Drop {
        src: u16,
    },
    Binary {
        dst: u16,
        op: BinaryOp,
        left: u16,
        right: u16,
    },
    /// Jumps when the register is falsy. The condition register is *not*
    /// consumed, matching `Op::JumpIfFalse`, which peeks.
    JumpIfFalsy {
        cond: u16,
        target: u32,
    },
    Jump {
        target: u32,
    },
    /// Calls `base`'s value with `argc` arguments held in the registers
    /// immediately above it, writing the result to `dst`. The callee re-enters
    /// the ordinary call path, so its errors propagate as `Result`.
    Call {
        dst: u16,
        base: u16,
        argc: u8,
    },
    Return {
        src: u16,
    },
}

#[derive(Clone)]
pub(super) struct CompactFunctionProgram {
    ops: Vec<CompactOp>,
    /// Total register file width: locals followed by former stack slots.
    register_count: usize,
    /// Locals this body reads through indexed storage. Entry declines unless
    /// the frame reports every one of them as authoritative.
    required_authoritative_slots: u128,
}

impl std::fmt::Debug for CompactFunctionProgram {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CompactFunctionProgram")
            .field("ops", &self.ops.len())
            .field("register_count", &self.register_count)
            .finish()
    }
}

/// Returns this body's compact program, compiling it on first use.
///
/// A body that cannot be represented caches `None`, so an unadmitted function
/// pays one `OnceCell` read per call rather than a repeated compile attempt.
pub(super) fn program_for(bytecode: &Bytecode) -> Option<&CompactFunctionProgram> {
    bytecode
        .compact_function_program
        .get_or_init(|| compile::compile(bytecode))
        .as_ref()
}

#[cfg(test)]
mod tests;
