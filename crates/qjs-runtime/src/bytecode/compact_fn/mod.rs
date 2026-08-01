//! Whole-function compact register execution.
//!
//! `Vm::run_current_activation` is one 24,792-instruction function whose
//! register allocator has given up: every dispatch reloads `self`, `self.ip`,
//! and the code pointer from the stack before any opcode does work. That
//! preamble is most of the per-instruction gap against QuickJS-NG, and it is
//! paid by all ninety-odd opcodes at once, so no per-opcode change can remove
//! it (`tasks/T021-single-vm-frame-stack.md`, 2026-08-01 root cause).
//!
//! This module is the other half of the answer that `typed_loop` gives for
//! loops: a small, separate executor that keeps its program counter and
//! register base in machine registers. Where `typed_loop` accelerates a *loop
//! region* with scalar registers, this accelerates a *whole function body*
//! with `Value` registers, which is what a recursive body needs -- its cost is
//! spread across an activation, not concentrated in a backedge.
//!
//! Deliberate boundaries, which are what make the tier safe:
//!
//! - **All-or-nothing admission.** A body is either fully representable or is
//!   not admitted at all. There is no deoptimization after entry, so there is
//!   no replay-after-side-effects problem to solve.
//! - **The calling convention is unchanged.** A call inside an admitted body
//!   re-enters the ordinary path and still builds a nested `Vm`. This tier
//!   removes generic dispatch, not frame construction; the explicit frame
//!   stack is a separate unit.
//! - **The operand stack becomes registers.** Stack slots are assigned to
//!   register indices at compile time, which is what removes the push/pop
//!   traffic that dominates a small body.

use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

use qjs_ast::BinaryOp;

use super::ir::Bytecode;

mod activation;
mod compile;
mod execute;

pub(super) use activation::try_run_standalone;

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
    /// Recycled register files. Deep recursion holds one per active frame, so
    /// this pools like the operand stack rather than keeping a single slot.
    scratch_pool: OnceCell<Rc<RefCell<Vec<Vec<crate::Value>>>>>,
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

impl CompactFunctionProgram {
    /// How many register files one body retains for reuse, mirroring the
    /// operand-stack recycler's bound so runaway depth cannot retain unbounded
    /// storage after it unwinds.
    const MAX_POOLED: usize = 32;

    fn take_registers(&self) -> Vec<crate::Value> {
        self.scratch_pool
            .get_or_init(|| Rc::new(RefCell::new(Vec::new())))
            .borrow_mut()
            .pop()
            .unwrap_or_default()
    }

    fn recycle_registers(&self, mut registers: Vec<crate::Value>) {
        // Reset in place rather than clearing. A cleared buffer has to be
        // grown again by the next activation, which showed up as
        // `Vec::extend_with` in the profile; keeping the length means the next
        // `take_registers` can use it as-is.
        //
        // `fill` would call `drop_in_place` per element. Most registers hold a
        // number by the time a body returns, so the same inline discriminant
        // test the executor uses for its stores pays here too.
        for slot in &mut registers {
            let previous = std::mem::replace(slot, crate::Value::Undefined);
            if matches!(
                previous,
                crate::Value::Number(_)
                    | crate::Value::Boolean(_)
                    | crate::Value::Null
                    | crate::Value::Undefined
            ) {
                std::mem::forget(previous);
            }
        }
        let mut pooled = self
            .scratch_pool
            .get_or_init(|| Rc::new(RefCell::new(Vec::new())))
            .borrow_mut();
        if pooled.len() < Self::MAX_POOLED {
            pooled.push(registers);
        }
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
