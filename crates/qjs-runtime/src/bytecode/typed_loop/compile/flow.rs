//! Selected-stream control-flow joins and branch targets.

use super::{Class, Op, Origin};

/// Joins two abstract stacks: equal depth and equal register class per entry,
/// keeping the weaker provenance. `None` when they cannot be joined.
pub(super) fn merge_states(
    left: &[(u16, Class, Origin)],
    right: &[(u16, Class, Origin)],
) -> Option<Vec<(u16, Class, Origin)>> {
    if left.len() != right.len() {
        return None;
    }
    left.iter()
        .zip(right)
        .map(|(left, right)| {
            (left.0 == right.0 && left.1 == right.1).then_some((
                left.0,
                left.1,
                if left.2 == right.2 {
                    left.2
                } else {
                    Origin::Computed
                },
            ))
        })
        .collect()
}

/// Which instructions of a region are the target of a jump inside it: those are
/// the joins, and an operand has to be in its depth's register at each one.
pub(super) fn is_target(code: &[Op], header: usize, backedge: usize) -> Vec<bool> {
    let mut targets = vec![false; backedge - header + 1];
    let mut mark = |target: usize| {
        if let Some(offset) = target.checked_sub(header)
            && offset < targets.len()
        {
            targets[offset] = true;
        }
    };
    for op in &code[header..=backedge.min(code.len() - 1)] {
        match op {
            Op::Jump(target) | Op::JumpIfFalse(target) | Op::JumpIfTrue(target) => mark(*target),
            Op::CompareLocalsJumpFalse { target, .. } => {
                mark(*target);
                mark(*target + 1);
            }
            Op::IncrementLocal {
                jump: Some(target), ..
            } => mark(*target),
            _ => {}
        }
    }
    targets
}
