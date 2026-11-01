//! Evaluation effects of encoded numeric operations, independent of liveness.
//! This whitelist relies on lowering's scalar-kind proofs: there are no boxed
//! values, property reads, TDZ reads or user coercions in a pure numeric op.
use super::NumOp;
use qjs_ast::BinaryOp;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Effect {
    Pure,
    /// Evaluation or callee admission must happen even when the result is dead.
    MustEvaluate,
    Control,
}

impl NumOp {
    pub(super) fn effect(&self) -> Effect {
        match self {
            Self::Const { .. }
            | Self::Move { .. }
            | Self::Add { .. }
            | Self::Sub { .. }
            | Self::Mul { .. }
            | Self::Div { .. }
            | Self::BitAnd { .. }
            | Self::BitOr { .. }
            | Self::BitXor { .. }
            | Self::Shl { .. }
            | Self::Shr { .. }
            | Self::UShr { .. }
            | Self::Lt { .. }
            | Self::Le { .. }
            | Self::Gt { .. }
            | Self::Ge { .. }
            | Self::Eq { .. }
            | Self::Ne { .. }
            | Self::Neg { .. }
            | Self::BitNot { .. }
            | Self::Not { .. }
            | Self::Nop => Effect::Pure,
            Self::Other {
                op: BinaryOp::Rem | BinaryOp::Pow,
                ..
            } => Effect::Pure,
            // Calls must resolve and validate their callees before a numeric
            // run is allowed. Otherwise the normal path must execute the JS
            // effects/throws. Native and unmodeled operations conservatively
            // retain that obligation, too; result liveness proves no purity.
            Self::Call { .. } | Self::Native { .. } | Self::Other { .. } => Effect::MustEvaluate,
            Self::Jump { .. }
            | Self::JumpIfFalsy { .. }
            | Self::JumpUnless { .. }
            | Self::JumpIfAndZero { .. }
            | Self::Return { .. }
            | Self::Bail => Effect::Control,
        }
    }
}
