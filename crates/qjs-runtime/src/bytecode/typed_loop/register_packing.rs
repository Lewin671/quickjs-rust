//! Register-file packing for a compiled loop region.
//!
//! The builder reserves the first `MAX_STACK_DEPTH` entries of each register
//! file so that an abstract operand at depth N always lands in register N.
//! Most regions use only a few depths, though, while their persistent locals,
//! globals and constants begin after the whole reservation. These routines
//! move the persistent registers down to sit immediately above the highest
//! stack register a region actually uses, and rewrite every reference to
//! match, before the runtime sizes its scratch files.

use super::{Class, MAX_REGISTERS, MAX_STACK_DEPTH, Typed, TypedOp};
use crate::Value;

pub(super) fn compact_scalar_registers(
    ops: &mut [TypedOp],
    site_entries: &mut [(Class, u16)],
    local_slots: &mut [(u16, u32)],
    written_locals: &mut [(u16, u32)],
    global_reads: &mut [(u16, String)],
    constants: &mut [(u16, Typed)],
    next_register: usize,
) -> Option<usize> {
    let stack_register_count = stack_register_count(ops, site_entries, Class::Scalar);
    remap_ops(ops, Class::Scalar, stack_register_count);
    remap_site_entries(site_entries, Class::Scalar, stack_register_count);
    remap_register_pairs(local_slots, stack_register_count);
    remap_register_pairs(written_locals, stack_register_count);
    remap_register_pairs(global_reads, stack_register_count);
    remap_register_pairs(constants, stack_register_count);
    compact_register_count(next_register, stack_register_count)
}

pub(super) struct BoxedRegisterMetadata<'a> {
    pub(super) locals: &'a mut [(u16, u32)],
    pub(super) numeric_native_callee_registers: &'a mut [u16],
    pub(super) written_locals: &'a mut [u16],
    pub(super) global_reads: &'a mut [(u16, String)],
    pub(super) constants: &'a mut [(u16, Value)],
}

pub(super) fn compact_boxed_registers(
    ops: &mut [TypedOp],
    site_entries: &mut [(Class, u16)],
    metadata: BoxedRegisterMetadata<'_>,
    next_boxed: usize,
) -> Option<usize> {
    let BoxedRegisterMetadata {
        locals,
        numeric_native_callee_registers,
        written_locals,
        global_reads,
        constants,
    } = metadata;
    let stack_register_count = stack_register_count(ops, site_entries, Class::Boxed);
    remap_ops(ops, Class::Boxed, stack_register_count);
    remap_site_entries(site_entries, Class::Boxed, stack_register_count);
    remap_register_pairs(locals, stack_register_count);
    for register in numeric_native_callee_registers {
        remap_register(register, stack_register_count);
    }
    for register in written_locals {
        remap_register(register, stack_register_count);
    }
    remap_register_pairs(global_reads, stack_register_count);
    remap_register_pairs(constants, stack_register_count);
    compact_register_count(next_boxed, stack_register_count)
}

fn stack_register_count(ops: &[TypedOp], site_entries: &[(Class, u16)], class: Class) -> usize {
    let mut count = site_entries
        .iter()
        .filter_map(|(candidate, register)| (*candidate == class).then_some(*register))
        .fold(0, note_stack_register);
    for op in ops {
        let mut copy = *op;
        visit_registers(&mut copy, class, |register| {
            count = note_stack_register(count, *register);
        });
    }
    count
}

fn remap_ops(ops: &mut [TypedOp], class: Class, stack_register_count: usize) {
    for op in ops {
        visit_registers(op, class, |register| {
            remap_register(register, stack_register_count)
        });
    }
}

fn note_stack_register(count: usize, register: u16) -> usize {
    let register = usize::from(register);
    if register < MAX_STACK_DEPTH {
        count.max(register + 1)
    } else {
        count
    }
}

fn visit_registers(op: &mut TypedOp, class: Class, mut visit: impl FnMut(&mut u16)) {
    match class {
        Class::Scalar => match op {
            TypedOp::Move { dst, src } | TypedOp::ToNumeric { dst, src } => {
                visit(dst);
                visit(src);
            }
            TypedOp::Binary {
                dst, left, right, ..
            } => {
                visit(dst);
                visit(left);
                visit(right);
            }
            TypedOp::Unary { dst, src, .. } | TypedOp::Update { dst, src, .. } => {
                visit(dst);
                visit(src);
            }
            TypedOp::DenseRead { dst, index, .. } => {
                visit(dst);
                visit(index);
            }
            TypedOp::DenseWrite { index, value, .. } => {
                visit(index);
                visit(value);
            }
            TypedOp::StoreSloppyGlobal { value, .. } => visit(value),
            TypedOp::JumpIfFalsy { cond, .. } | TypedOp::Exit { cond, .. } => visit(cond),
            TypedOp::Unbox { dst, .. } => visit(dst),
            TypedOp::Box { src, .. } => visit(src),
            TypedOp::ElementRead { index, .. } => visit(index),
            TypedOp::CallNumericNative {
                dst, first, second, ..
            } => {
                visit(dst);
                visit(first);
                visit(second);
            }
            // Only the arguments are scalar; the receiver, callee, and result
            // are boxed because a user callee takes and returns any value.
            TypedOp::CallClosedFormLeaf { first, second, .. } => {
                visit(first);
                visit(second);
            }
            // Every operand of a computed access is boxed.
            TypedOp::Jump { .. }
            | TypedOp::MoveBoxed { .. }
            | TypedOp::GetNamed { .. }
            | TypedOp::SetNamed { .. }
            | TypedOp::ComputedRead { .. }
            | TypedOp::ComputedWrite { .. } => {}
        },
        Class::Boxed => match op {
            TypedOp::MoveBoxed { dst, src } => {
                visit(dst);
                visit(src);
            }
            TypedOp::Unbox { src, .. } => visit(src),
            TypedOp::Box { dst, .. } => visit(dst),
            TypedOp::GetNamed { dst, object, .. } => {
                visit(dst);
                visit(object);
            }
            TypedOp::SetNamed { object, value, .. } => {
                visit(object);
                visit(value);
            }
            TypedOp::ComputedRead { dst, receiver, key } => {
                visit(dst);
                visit(receiver);
                visit(key);
            }
            TypedOp::ComputedWrite {
                receiver,
                key,
                value,
            } => {
                visit(receiver);
                visit(key);
                visit(value);
            }
            TypedOp::ElementRead { dst, receiver, .. } => {
                visit(dst);
                visit(receiver);
            }
            TypedOp::CallNumericNative { callee, .. } => visit(callee),
            TypedOp::CallClosedFormLeaf {
                dst,
                receiver,
                callee,
                ..
            } => {
                visit(dst);
                visit(receiver);
                visit(callee);
            }
            TypedOp::Move { .. }
            | TypedOp::ToNumeric { .. }
            | TypedOp::Binary { .. }
            | TypedOp::Unary { .. }
            | TypedOp::Update { .. }
            | TypedOp::DenseRead { .. }
            | TypedOp::DenseWrite { .. }
            | TypedOp::StoreSloppyGlobal { .. }
            | TypedOp::JumpIfFalsy { .. }
            | TypedOp::Jump { .. }
            | TypedOp::Exit { .. } => {}
        },
    }
}

fn remap_site_entries(
    site_entries: &mut [(Class, u16)],
    class: Class,
    stack_register_count: usize,
) {
    for (candidate, register) in site_entries {
        if *candidate == class {
            remap_register(register, stack_register_count);
        }
    }
}

fn remap_register_pairs<T>(pairs: &mut [(u16, T)], stack_register_count: usize) {
    for (register, _) in pairs {
        remap_register(register, stack_register_count);
    }
}

fn remap_register(register: &mut u16, stack_register_count: usize) {
    let register_index = usize::from(*register);
    if register_index >= MAX_STACK_DEPTH {
        let compacted = stack_register_count + register_index - MAX_STACK_DEPTH;
        debug_assert!(compacted < MAX_REGISTERS);
        *register = compacted as u16;
    }
}

fn compact_register_count(next_register: usize, stack_register_count: usize) -> Option<usize> {
    stack_register_count.checked_add(next_register.checked_sub(MAX_STACK_DEPTH)?)
}
