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

/// A compiled numeric plan cannot call JavaScript or write a captured cell.
/// Failed operand guards still deoptimize before executing the source body.
/// This proof does not retain or freeze any captured values.
pub(in crate::bytecode) fn replayable_numeric_plan(bytecode: &Bytecode) -> bool {
    bytecode
        .numeric_leaf_plan
        .get_or_init(|| NumericLeafPlan::compile(bytecode))
        .as_ref()
        .is_some_and(|plan| match plan {
            NumericLeafPlan::NumberOnly(_) => true,
            NumericLeafPlan::General(plan) => plan.pure,
        })
}

/// The existing pure shortcut evaluated on already unboxed Number arguments.
/// All effect and layout guards are shared with ordinary pure leaf evaluation.
/// The caller still owes ordinary function-kind admission.
#[inline]
pub(in crate::bytecode) fn try_eval_pure_number_shortcut(
    bytecode: &Bytecode,
    params: &FunctionParams,
    argument_number: impl Fn(usize) -> Option<f64>,
    upvalues: &[BindingCell],
) -> Option<Value> {
    let plan = bytecode
        .numeric_leaf_plan
        .get_or_init(|| NumericLeafPlan::compile(bytecode))
        .as_ref()?;
    let NumericLeafPlan::General(plan) = plan else {
        return None;
    };
    if !plan.pure
        || bytecode.parameter_slots().len() != params.positional.len()
        || bytecode.received_upvalue_slots().len() != upvalues.len()
    {
        return None;
    }
    plan.shortcut
        .as_ref()?
        .eval_numbers_with(argument_number, upvalues)
}

#[cfg(test)]
mod number_view_tests {
    use super::*;

    #[test]
    fn unboxed_arguments_share_shortcut_results_and_effect_admission() {
        let script = qjs_parser::parse_script("function f(a,b){return a+b;} function outer(){var x=1;return function g(a){x+=a;return x;}}").unwrap();
        let root = crate::bytecode::compiler::compile_script(&script).unwrap();
        let (params, code) = root
            .code
            .iter()
            .find_map(|op| {
                if let Op::NewFunction {
                    name,
                    params,
                    bytecode,
                    ..
                } = op
                    && name.as_deref() == Some("f")
                {
                    Some((params, bytecode))
                } else {
                    None
                }
            })
            .unwrap();
        for (a, b) in [
            (2.0, 3.0),
            (-0.0, -0.0),
            (f64::INFINITY, 1.0),
            (f64::NAN, 1.0),
        ] {
            let expected = try_eval_pure_numeric_leaf(
                code,
                params,
                &[Value::Number(a), Value::Number(b)],
                &[],
            )
            .unwrap();
            let actual = try_eval_pure_number_shortcut(
                code,
                params,
                |index| [a, b].get(index).copied(),
                &[],
            )
            .unwrap();
            assert!(actual.same_value(&expected));
        }
        let outer = root
            .code
            .iter()
            .find_map(|op| {
                if let Op::NewFunction { name, bytecode, .. } = op
                    && name.as_deref() == Some("outer")
                {
                    Some(bytecode)
                } else {
                    None
                }
            })
            .unwrap();
        let (params, code) = outer
            .code
            .iter()
            .find_map(|op| {
                if let Op::NewFunction {
                    params, bytecode, ..
                } = op
                {
                    Some((params, bytecode))
                } else {
                    None
                }
            })
            .unwrap();
        let cell = BindingCell::new(Value::Number(1.0));
        assert!(
            try_eval_pure_number_shortcut(
                code,
                params,
                |index| [3.0].get(index).copied(),
                std::slice::from_ref(&cell)
            )
            .is_none()
        );
        assert_eq!(cell.read(), Ok(Value::Number(1.0)));
    }
}
