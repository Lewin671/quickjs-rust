//! Pure numeric leaves may be probed or retried; captured writes may not.
use super::*;

/// Whether no compiled op writes a received cell: the plan's purity, decided
/// once when it is compiled.
pub(super) fn ops_are_pure(ops: &[FastOp]) -> bool {
    !ops.iter().any(|op| match op {
        FastOp::StoreLocal {
            upvalue_index: Some(_),
            ..
        }
        | FastOp::UpdateUpvalueConstReturn { .. } => true,
        FastOp::LoadConst(_)
        | FastOp::LoadLocal(_)
        | FastOp::LoadLocalOrUndefined(_)
        | FastOp::StoreLocal {
            upvalue_index: None,
            ..
        }
        | FastOp::Dup
        | FastOp::Pop
        | FastOp::ToNumeric
        | FastOp::Update(_)
        | FastOp::Binary(_)
        | FastOp::BinaryConstRight(_, _)
        | FastOp::Return
        | FastOp::ReturnConst(_) => false,
    })
}

/// Replayable admission/evaluation for loop helper probes and speculative calls.
/// Ordinary leaf evaluation can commit received-cell writes at Return; success
/// therefore does not prove purity. Use the shared effect policy before either
/// probing a body or running one under a deoptimization/retry protocol.
pub(in crate::bytecode) fn try_eval_pure_numeric_leaf(
    bytecode: &Bytecode,
    params: &FunctionParams,
    arguments: &[Value],
    upvalues: &[BindingCell],
) -> Option<Value> {
    let plan = bytecode
        .numeric_leaf_plan
        .get_or_init(|| NumericLeafPlan::compile(bytecode))
        .as_ref()?;
    let pure = match plan {
        NumericLeafPlan::NumberOnly(_) => true,
        NumericLeafPlan::General(plan) => plan.pure,
    };
    if !pure {
        return None;
    }
    eval_numeric_leaf_plan(plan, bytecode, params, arguments, upvalues)
}
