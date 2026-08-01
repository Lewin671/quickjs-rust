//! Slot-authority peepholes: instruction fusions that need only the control
//! flow graph and one slot's static assignment authority.
//!
//! They live beside the virtual-object lowering because they share its
//! analysis and its same-length rewritten stream, but they do not depend on
//! any virtual candidate. Keeping them separate is what lets a body whose
//! candidate analysis did not complete still receive them.

use std::rc::Rc;

use crate::bytecode::ir::{Bytecode, Op};

use super::lower::{VirtualObjectProgram, original_program, peephole_variant};

/// The stream for a body whose virtual-object candidate analysis did not
/// complete.
///
/// The increment fusion needs only the control flow graph and one slot's
/// static assignment authority, both of which an incomplete analysis still
/// carries. Returning the unmodified stream here meant an ordinary `for` loop
/// in such a body dispatched seven instructions for `i++` where the same loop
/// in a body with a complete analysis dispatched one, for a reason unrelated
/// to virtual objects -- a `try` block anywhere in the function is enough to
/// land here.
///
/// Only the increment fusion is applied. `fuse_local_copies` asks a weaker
/// question -- `is_authoritative` rather than `is_assignment_authoritative` --
/// and a module body's imported bindings answer it, so lowering them here
/// copied a live temporal-dead-zone marker straight out of a frame slot
/// instead of observing it through `load_local`. Three module tests catch
/// that. The increment fusion requires a hoisted mutable binding and declines
/// at runtime on anything that is not already a number, so it has no such
/// exposure.
///
/// The variant requires no authoritative slots of its own: the fusion already
/// proved the slot it rewrites is statically authoritative, which does not
/// vary between frames.
pub(super) fn peephole_only_program(
    bytecode: &Bytecode,
    analysis: &super::VirtualObjectAnalysis,
) -> VirtualObjectProgram {
    let mut code: Vec<Op> = bytecode.code.to_vec();
    fuse_local_increments(bytecode, analysis, &mut code);
    if !code
        .iter()
        .any(|op| matches!(op, Op::IncrementLocal { .. }))
    {
        return original_program();
    }
    peephole_variant(Rc::from(code))
}

pub(super) fn fuse_local_increments(
    bytecode: &Bytecode,
    analysis: &super::VirtualObjectAnalysis,
    code: &mut [Op],
) {
    for ip in 0..code.len().saturating_sub(5) {
        let (
            Op::LoadLocal(slot),
            Op::ToNumeric,
            Op::Dup,
            Op::Update(qjs_ast::UpdateOp::Increment),
            Op::AssignLocal(assigned),
            Op::Pop,
        ) = (
            &code[ip],
            &code[ip + 1],
            &code[ip + 2],
            &code[ip + 3],
            &code[ip + 4],
            &code[ip + 5],
        )
        else {
            continue;
        };
        if slot == assigned
            && analysis
                .slot_authority
                .is_assignment_authoritative(bytecode, *slot)
            && range_is_linear(analysis, ip, ip + 5)
        {
            let jump = match code.get(ip + 6) {
                Some(Op::Jump(target)) => Some(*target),
                _ => None,
            };
            code[ip] = Op::IncrementLocal {
                slot: *slot,
                skip: 5,
                jump,
            };
        }
    }
}
pub(super) fn fuse_local_copies(analysis: &super::VirtualObjectAnalysis, code: &mut [Op]) {
    for ip in 0..code.len().saturating_sub(1) {
        let (Op::LoadLocal(from), Op::StoreLocal(to)) = (&code[ip], &code[ip + 1]) else {
            continue;
        };
        if analysis.slot_authority.is_authoritative(*from)
            && analysis.slot_authority.is_authoritative(*to)
            && range_is_linear(analysis, ip, ip + 1)
        {
            code[ip] = Op::CopyLocal {
                from: *from,
                to: *to,
                skip: 1,
            };
        }
    }
}
pub(super) fn range_is_linear(
    analysis: &super::VirtualObjectAnalysis,
    start: usize,
    end: usize,
) -> bool {
    analysis
        .cfg
        .blocks
        .iter()
        .any(|block| block.start <= start && end < block.end)
}
