//! Which bytecode operations a typed region admits, decided by inspection
//! before anything is lowered.

use qjs_ast::{BinaryOp, UnaryOp};

use crate::bytecode::ir::Op;

/// Index expressions are replayed from the surrounding assignment's entry if
/// a later typed operation declines. They therefore cannot contain a write or
/// branch that would become observable twice. The admitted scalar operations
/// themselves either operate on Numbers/booleans or decline before invoking
/// any user code.
pub(super) fn scalar_expression_may_write_or_branch(op: &Op) -> bool {
    expression_has_control_flow(op)
        || matches!(
            op,
            Op::AppendStringLiteralLocal { .. }
                | Op::AppendStringLiteralGlobal { .. }
                | Op::StoreLocal(_)
                | Op::AssignLocal(_)
                | Op::ClearLocal(_)
                | Op::DefineGlobalVar(_)
                | Op::StoreGlobalStrict(_)
                | Op::StoreGlobalSloppy { .. }
                | Op::StoreLocalOrGlobalSloppy { .. }
                | Op::StoreIdentWith { .. }
                | Op::StoreResolvedIdentWith { .. }
                | Op::SetProp { .. }
                | Op::SetPropNamed { .. }
                | Op::SetPropIndex { .. }
                | Op::SetPrivate(_)
                | Op::DeleteProp { .. }
                | Op::DeleteIdent(_)
                | Op::DeleteIdentWith { .. }
                | Op::IncrementLocal { .. }
                | Op::CopyLocal { .. }
                | Op::BinaryAssignLocals { .. }
        )
}

pub(super) fn expression_has_control_flow(op: &Op) -> bool {
    matches!(
        op,
        Op::Jump(_)
            | Op::JumpIfFalse(_)
            | Op::JumpIfTrue(_)
            | Op::JumpIfNotNullish(_)
            | Op::AbruptJump(_)
            | Op::CompareLocalsJumpFalse { .. }
    )
}

pub(in crate::bytecode::typed_loop) fn admitted_binary(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::Pow
            | BinaryOp::Shl
            | BinaryOp::Shr
            | BinaryOp::UShr
            | BinaryOp::BitwiseAnd
            | BinaryOp::BitwiseOr
            | BinaryOp::BitwiseXor
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::StrictEq
            | BinaryOp::StrictNe
    )
}

pub(in crate::bytecode::typed_loop) fn admitted_unary(op: UnaryOp) -> bool {
    matches!(
        op,
        UnaryOp::Minus | UnaryOp::Plus | UnaryOp::BitwiseNot | UnaryOp::Not
    )
}
