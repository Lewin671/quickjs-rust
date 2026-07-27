//! Marker selection for a lowered, constant virtual literal followed by a
//! fused binary assignment.
//!
//! `InitVirtualObject::slot` is inert for a zero-input initializer. Reserving
//! this otherwise-unused value lets the VM recognize the compound operation
//! without inspecting the following bytecodes on every ordinary virtual
//! literal.

use crate::bytecode::ir::Op;

pub(in crate::bytecode) const VIRTUAL_CONSTANT_BINARY_INIT_SLOT: usize = usize::MAX;

pub(super) fn mark_inits_for_constant_binary_assignments(code: &mut [Op]) {
    for ip in 0..code.len() {
        let (count, local, skip) = match code.get(ip) {
            Some(Op::InitVirtualObject {
                count, local, skip, ..
            }) => (*count, *local, *skip),
            _ => continue,
        };
        if count != 0 || local.is_none() {
            continue;
        }
        let Some(first) = ip.checked_add(skip).and_then(|ip| ip.checked_add(1)) else {
            continue;
        };
        let Some(binary) = first
            .checked_add(1)
            .and_then(|right| match code.get(right) {
                Some(Op::LoadConst(_)) => right.checked_add(1),
                Some(Op::LoadVirtualNumber { skip, .. }) => right
                    .checked_add(1)
                    .and_then(|after_load| after_load.checked_add(*skip)),
                _ => None,
            })
        else {
            continue;
        };
        if !matches!(
            (code.get(first), code.get(binary)),
            (Some(Op::LoadLocal(_)), Some(Op::BinaryAssignLocals { .. }))
        ) {
            continue;
        }
        if let Some(Op::InitVirtualObject { slot, .. }) = code.get_mut(ip) {
            *slot = VIRTUAL_CONSTANT_BINARY_INIT_SLOT;
        }
    }
}
