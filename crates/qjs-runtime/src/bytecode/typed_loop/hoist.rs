//! Loop-invariant property reads, read once on entry.
//!
//! `for (i = 0; i < this.length; i++) if (this[i].pos == obj.pos) ...` reads
//! `this.length` and `obj.pos` on every iteration although neither can
//! change while the loop runs: the receivers are locals (or globals) the
//! region never writes, and the region stores nothing that could change them
//! -- no element or global store, no push, no call, and no named write of
//! the same name.
//! Such a read is performed once when the loop is entered, into a register
//! of its own, and the read inside the loop becomes a copy of it. A read that
//! would not succeed on entry -- an accessor, an exotic receiver -- declines
//! the entry, before anything has run.

use std::collections::BTreeSet;

use super::{MAX_REGISTERS, TypedOp};

/// Rewrites `ops` and returns the reads to perform on entry, each into a
/// register added past `register_count` or `boxed_count`. `invariant` names
/// the boxed registers seeded on entry that the region reads but never
/// writes through a local store.
pub(super) fn hoist_invariant_reads(
    ops: &mut [TypedOp],
    names: &[std::rc::Rc<str>],
    invariant: impl IntoIterator<Item = u16>,
    register_count: &mut usize,
    boxed_count: &mut usize,
) -> Vec<TypedOp> {
    if ops.iter().any(writes_or_calls) {
        return Vec::new();
    }
    // A named write only overwrites an existing own data property (the
    // tier's `set_named` runs no setter and adds no property), so it can
    // change no read of any other name.
    // Names are compared as text: two sites naming one property may hold
    // distinct name indices.
    let name_of = |index: u16| names.get(usize::from(index)).map(|name| &**name);
    let written_names: BTreeSet<&str> = ops
        .iter()
        .filter_map(|op| match *op {
            TypedOp::SetNamed { name, .. } | TypedOp::SetNamedTyped { name, .. } => name_of(name),
            _ => None,
        })
        .collect();
    let unwritten = |name: u16| name_of(name).is_some_and(|name| !written_names.contains(name));
    let written: BTreeSet<u16> = ops.iter().filter_map(boxed_destination).collect();
    let invariant: BTreeSet<u16> = invariant
        .into_iter()
        .filter(|register| !written.contains(register))
        .collect();
    let jump_targets: BTreeSet<usize> = ops
        .iter()
        .filter_map(|op| match *op {
            TypedOp::Jump { target } | TypedOp::JumpIfFalsy { target, .. } => {
                usize::try_from(target).ok()
            }
            _ => None,
        })
        .collect();
    let mut hoisted = Vec::new();
    for index in 0..ops.len() {
        // A receiver copied into a stack register by the operation just
        // before -- `this.length` loads `this` first -- is still the invariant
        // register's value when control can only arrive from that copy.
        let copied_from = match (index.checked_sub(1).map(|at| ops[at]), &ops[index]) {
            (
                Some(TypedOp::MoveBoxed { dst, src }),
                TypedOp::GetNamed { object, .. } | TypedOp::GetNamedTyped { object, .. },
            ) if dst == *object && invariant.contains(&src) && !jump_targets.contains(&index) => {
                Some(src)
            }
            _ => None,
        };
        let op = &mut ops[index];
        match *op {
            TypedOp::GetNamed {
                dst,
                object,
                name,
                cache,
            } if (invariant.contains(&object) || copied_from.is_some())
                && unwritten(name)
                && *boxed_count < MAX_REGISTERS =>
            {
                let object = copied_from.unwrap_or(object);
                let Ok(register) = u16::try_from(*boxed_count) else {
                    break;
                };
                *boxed_count += 1;
                hoisted.push(TypedOp::GetNamed {
                    dst: register,
                    object,
                    name,
                    cache,
                });
                *op = TypedOp::MoveBoxed { dst, src: register };
            }
            TypedOp::GetNamedTyped {
                dst,
                object,
                name,
                cache,
            } if (invariant.contains(&object) || copied_from.is_some())
                && unwritten(name)
                && *register_count < MAX_REGISTERS =>
            {
                let object = copied_from.unwrap_or(object);
                let Ok(register) = u16::try_from(*register_count) else {
                    break;
                };
                *register_count += 1;
                hoisted.push(TypedOp::GetNamedTyped {
                    dst: register,
                    object,
                    name,
                    cache,
                });
                *op = TypedOp::Move { dst, src: register };
            }
            _ => {}
        }
    }
    hoisted
}

/// Whether `op` stores through anything but a known name, or calls code
/// that could store.
fn writes_or_calls(op: &TypedOp) -> bool {
    matches!(
        op,
        TypedOp::DenseWrite { .. }
            | TypedOp::StoreSloppyGlobal { .. }
            | TypedOp::ComputedWrite { .. }
            | TypedOp::ArrayPush { .. }
            | TypedOp::CallClosedFormLeaf { .. }
    )
}

/// The boxed register `op` writes, if any.
fn boxed_destination(op: &TypedOp) -> Option<u16> {
    match *op {
        TypedOp::MoveBoxed { dst, .. }
        | TypedOp::Box { dst, .. }
        | TypedOp::GetNamed { dst, .. }
        | TypedOp::ElementRead { dst, .. }
        | TypedOp::ComputedRead { dst, .. }
        | TypedOp::CallClosedFormLeaf { dst, .. } => Some(dst),
        _ => None,
    }
}
