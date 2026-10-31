//! Entry proofs for named-read invariance across native calls. Guard inputs
//! are immutable activation registers, never values retained in IC metadata.
use super::{BOXED_ARGUMENTS, TypedOp};
use crate::{NativeFunction, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub(super) struct Guard {
    callee: u16,
    receiver: Option<u16>,
}
/// Derive entry-register provenance through copies, discarding temporary
/// aliases at every control-flow target. Unknown calls keep the normal region.
pub(super) fn guards(
    ops: &[TypedOp],
    invariant: &[u16],
    hoisted: &[TypedOp],
) -> Option<Vec<Guard>> {
    let stable: BTreeSet<u16> = invariant
        .iter()
        .copied()
        .chain(hoisted.iter().filter_map(|op| {
            if let TypedOp::GetNamed { dst, .. } = *op {
                Some(dst)
            } else {
                None
            }
        }))
        .collect();
    let targets: BTreeSet<usize> = ops
        .iter()
        .filter_map(|op| match *op {
            TypedOp::Jump { target } | TypedOp::JumpIfFalsy { target, .. } => {
                usize::try_from(target).ok()
            }
            _ => None,
        })
        .collect();
    let mut aliases: BTreeMap<u16, u16> = stable.iter().map(|&r| (r, r)).collect();
    let mut guards = Vec::new();
    for (index, op) in ops.iter().enumerate() {
        if targets.contains(&index) {
            aliases.retain(|r, _| stable.contains(r));
        }
        if let TypedOp::CallClosedFormLeaf {
            callee,
            receiver,
            arity,
            ..
        } = *op
        {
            if arity & BOXED_ARGUMENTS != 0 {
                return None;
            }
            guards.push(Guard {
                callee: *aliases.get(&callee)?,
                receiver: aliases.get(&receiver).copied(),
            });
        }
        let source = if let TypedOp::MoveBoxed { src, .. } = *op {
            aliases.get(&src).copied()
        } else {
            None
        };
        if let Some(dst) = super::hoist::boxed_destination(op) {
            aliases.remove(&dst);
            if let Some(source) = source {
                aliases.insert(dst, source);
            }
        }
    }
    Some(guards)
}
impl Guard {
    /// Scalar arguments cannot call ToPrimitive. The selected implementations
    /// either ignore `this`, or read an immutable primitive string. Bound/user
    /// functions and operations with protocols, callbacks or writes fail here.
    pub(super) fn holds(&self, boxed: &[Value]) -> bool {
        let Some(Value::Function(function)) = boxed.get(self.callee as usize) else {
            return false;
        };
        if function.bound.is_some() {
            return false;
        }
        match function.native {
            Some(
                NativeFunction::MathAbs
                | NativeFunction::MathAcos
                | NativeFunction::MathAcosh
                | NativeFunction::MathAsin
                | NativeFunction::MathAsinh
                | NativeFunction::MathAtan
                | NativeFunction::MathAtan2
                | NativeFunction::MathAtanh
                | NativeFunction::MathCbrt
                | NativeFunction::MathCeil
                | NativeFunction::MathClz32
                | NativeFunction::MathCos
                | NativeFunction::MathCosh
                | NativeFunction::MathExp
                | NativeFunction::MathExpm1
                | NativeFunction::MathF16round
                | NativeFunction::MathFloor
                | NativeFunction::MathFround
                | NativeFunction::MathHypot
                | NativeFunction::MathImul
                | NativeFunction::MathLog
                | NativeFunction::MathLog1p
                | NativeFunction::MathLog10
                | NativeFunction::MathLog2
                | NativeFunction::MathMax
                | NativeFunction::MathMin
                | NativeFunction::MathPow
                | NativeFunction::MathRound
                | NativeFunction::MathSign
                | NativeFunction::MathSin
                | NativeFunction::MathSinh
                | NativeFunction::MathSqrt
                | NativeFunction::MathTan
                | NativeFunction::MathTanh
                | NativeFunction::MathTrunc,
            ) => true,
            Some(
                NativeFunction::StringPrototypeSlice
                | NativeFunction::StringPrototypeSubstr
                | NativeFunction::StringPrototypeSubstring
                | NativeFunction::StringPrototypeAt
                | NativeFunction::StringPrototypeCharAt
                | NativeFunction::StringPrototypeCharCodeAt
                | NativeFunction::StringPrototypeCodePointAt,
            ) => self
                .receiver
                .is_some_and(|r| matches!(boxed.get(r as usize), Some(Value::String(_)))),
            _ => false,
        }
    }
}
