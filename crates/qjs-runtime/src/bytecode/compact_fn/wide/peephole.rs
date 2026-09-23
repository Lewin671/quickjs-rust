//! Peephole rewrites the lowering applies as it emits operations.
//!
//! The lowering maps the interpreter's stack code onto registers one
//! instruction at a time, so an expression's value travels through a stack
//! register on its way to where it is used: a comparison materializes a
//! boolean only for a conditional jump to test and discard it, and a store
//! copies a result the producing operation could have written in place.
//! These rewrites fold such pairs while the operations are emitted. Each
//! applies only where control reaches the folded instruction solely from
//! the one before it, and the instructions it removes are marked as places
//! the tier cannot resume at.

use qjs_ast::BinaryOp;

use super::WideOp;
use crate::bytecode::ir::Op;

/// Whether the last operation emitted is the only one instruction `ip - 1`
/// produced and control reaches `ip` only from it.
pub(super) fn sole_op_of_previous(
    ip: usize,
    jump_targets: &[bool],
    compact_index: &[u32],
    ops: &[WideOp],
) -> bool {
    ip > 0
        && !jump_targets[ip]
        && !ops.is_empty()
        && compact_index[ip - 1] as usize == ops.len() - 1
}

/// Makes `op`, which writes its result to the stack register `from`, write
/// it to `to` instead, for an operation whose operands are read before the
/// result is written and whose `from` register holds no reference afterward
/// that the original would have released.
pub(super) fn retarget(op: &mut WideOp, from: u16, to: u16) -> bool {
    let dst = match op {
        WideOp::Move { dst, .. }
        | WideOp::LoadConst { dst, .. }
        | WideOp::LoadUpvalueLocal { dst, .. }
        | WideOp::LoadThis { dst }
        | WideOp::LoadGlobal { dst, .. }
        | WideOp::GetPropThis { dst, .. }
        | WideOp::Binary { dst, .. }
        | WideOp::Typeof { dst, .. }
        | WideOp::Unary { dst, .. } => dst,
        // A named read of a local receiver leaves no register behind; a
        // plain one would keep its receiver in `from`.
        WideOp::GetPropNamed { dst, obj, .. } if *obj != from => dst,
        _ => return false,
    };
    if *dst != from {
        return false;
    }
    *dst = to;
    true
}

/// A comparison, its conditional jump and the condition's pops, fused.
pub(super) struct FusedCompare {
    /// The first wide operation the fusion replaces, and its instruction.
    pub(super) first: usize,
    pub(super) first_ip: usize,
    /// Operations of the group that stay, before the fused one: a constant
    /// operand's load.
    pub(super) kept: Vec<WideOp>,
    pub(super) op: BinaryOp,
    pub(super) left: u16,
    pub(super) right: u16,
}

/// Whether `JumpIfFalse` at `ip` ends `left op right` with `op` relational
/// or equality, and both successors pop the condition: then one
/// `CompareJump` replaces the comparison, the jump and the fall-through
/// `Pop`, and jumps past the target's `Pop`. An operand loaded from a local
/// just before is read in place. Nothing inside the group may be a jump
/// target, so control reaches it only from its start.
pub(super) fn fuse_compare_jump(
    code: &[Op],
    ip: usize,
    target: usize,
    cond: u16,
    jump_targets: &[bool],
    compact_index: &[u32],
    ops: &[WideOp],
) -> Option<FusedCompare> {
    const COMPARISONS: [BinaryOp; 8] = [
        BinaryOp::Lt,
        BinaryOp::Le,
        BinaryOp::Gt,
        BinaryOp::Ge,
        BinaryOp::Eq,
        BinaryOp::Ne,
        BinaryOp::StrictEq,
        BinaryOp::StrictNe,
    ];
    let Op::Binary(op) = code.get(ip.checked_sub(1)?)? else {
        return None;
    };
    if !COMPARISONS.contains(op)
        || jump_targets[ip]
        || jump_targets[ip - 1]
        || jump_targets.get(ip + 1) != Some(&false)
        || !matches!(code.get(ip + 1), Some(Op::Pop))
        || !matches!(code.get(target), Some(Op::Pop))
        || target + 1 >= code.len()
        || code.len() >= usize::from(u16::MAX)
    {
        return None;
    }
    let binary = ops.len().checked_sub(1)?;
    let WideOp::Binary {
        dst,
        left,
        right,
        op: emitted,
    } = ops[binary]
    else {
        return None;
    };
    if emitted != *op || dst != cond || left != cond || compact_index[ip - 1] as usize != binary {
        return None;
    }
    // The operation emitted for bytecode instruction `at`, when it is the
    // only one and a plain local load into `register`.
    let local_load = |at: usize, index: usize, register: u16| -> Option<u16> {
        match (code.get(at)?, ops.get(index)?) {
            (Op::LoadLocal(_), WideOp::Move { dst, src })
                if *dst == register && compact_index[at] as usize == index =>
            {
                Some(*src)
            }
            _ => None,
        }
    };
    let mut fused = FusedCompare {
        first: binary,
        first_ip: ip - 1,
        kept: Vec::new(),
        op: *op,
        left,
        right,
    };
    // `a op b` and `a op <constant>` load their operands in the two
    // instructions before the comparison.
    if ip >= 3 && !jump_targets[ip - 2] && binary >= 2 {
        let right_load = local_load(ip - 2, binary - 1, right);
        let left_load = local_load(ip - 3, binary - 2, left);
        let right_is_constant = matches!(
            (&code[ip - 2], &ops[binary - 1]),
            (Op::LoadConst(_), WideOp::LoadConst { dst, .. }) if *dst == right
        ) && compact_index[ip - 2] as usize == binary - 1;
        match (left_load, right_load) {
            (Some(left), Some(right)) => {
                fused.first = binary - 2;
                fused.first_ip = ip - 3;
                fused.left = left;
                fused.right = right;
            }
            (Some(left), None) if right_is_constant => {
                fused.first = binary - 2;
                fused.first_ip = ip - 3;
                fused.kept.push(ops[binary - 1]);
                fused.left = left;
            }
            _ => {}
        }
    }
    Some(fused)
}
