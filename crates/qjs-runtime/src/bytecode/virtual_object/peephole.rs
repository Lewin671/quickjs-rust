//! Slot-authority peepholes: instruction fusions that need only the control
//! flow graph and one slot's static assignment authority.
//!
//! They live beside the virtual-object lowering because they share its
//! analysis and its same-length rewritten stream, but they do not depend on
//! any virtual candidate. Keeping them separate is what lets a body whose
//! candidate analysis did not complete still receive them.

use crate::bytecode::ir::{Bytecode, Op};

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
