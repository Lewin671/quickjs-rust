//! Pure numeric leaves may be probed or retried; captured writes may not.
use super::*;

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
    use super::super::typed_loop::helper_graph::numeric::effects::Effect;
    let plan = bytecode
        .numeric_leaf_plan
        .get_or_init(|| NumericLeafPlan::compile(bytecode))
        .as_ref()?;
    let effect = match plan {
        NumericLeafPlan::NumberOnly(_) => Effect::Pure,
        NumericLeafPlan::General(plan) => {
            if plan.ops.iter().any(|op| match op {
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
            }) {
                Effect::MustEvaluate
            } else {
                Effect::Pure
            }
        }
    };
    if effect != Effect::Pure {
        return None;
    }
    try_eval_numeric_leaf(bytecode, params, arguments, upvalues)
}
