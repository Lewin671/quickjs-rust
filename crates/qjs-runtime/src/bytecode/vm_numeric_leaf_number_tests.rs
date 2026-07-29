use super::super::vm_props::fast_number_binary_numbers;
use super::*;
use crate::bytecode::compiler;

#[test]
fn number_only_program_admits_local_temporary_bitwise_leaf() {
    let script = qjs_parser::parse_script(
        "function safeAdd(x, y) { \
            var low = (x & 65535) + (y & 65535); \
            var high = (x >> 16) + (y >> 16) + (low >> 16); \
            return (high << 16) | (low & 65535); \
        }",
    )
    .expect("source should parse");
    let script_bytecode = compiler::compile_script(&script).expect("source should compile");
    let function_bytecode = script_bytecode
        .code
        .iter()
        .find_map(|op| match op {
            Op::NewFunction { bytecode, .. } => Some(bytecode),
            _ => None,
        })
        .expect("function bytecode should be nested in the script");
    let plan = NumericLeafPlan::compile(function_bytecode).expect("leaf should be admitted");
    assert!(
        matches!(
            plan.shortcut,
            Some(NumericLeafShortcut::NumberOnlyProgram(_))
        ),
        "unexpected shortcut: {:#?}; ops: {:#?}",
        plan.shortcut,
        plan.ops
    );
    assert_eq!(
        plan.shortcut
            .as_ref()
            .and_then(|shortcut| shortcut
                .eval(&[Value::Number(2_147_483_647.0), Value::Number(1.0)], &[],)),
        Some(Value::Number(-2_147_483_648.0))
    );
    assert!(
        plan.shortcut
            .as_ref()
            .and_then(
                |shortcut| shortcut.eval(&[Value::Number(1.0), Value::String("2".into())], &[],)
            )
            .is_none()
    );
}

#[test]
fn number_only_binary_matches_the_number_fast_path_at_edges() {
    let operations = [
        qjs_ast::BinaryOp::Add,
        qjs_ast::BinaryOp::Sub,
        qjs_ast::BinaryOp::Mul,
        qjs_ast::BinaryOp::Div,
        qjs_ast::BinaryOp::Rem,
        qjs_ast::BinaryOp::Pow,
        qjs_ast::BinaryOp::Shl,
        qjs_ast::BinaryOp::Shr,
        qjs_ast::BinaryOp::UShr,
        qjs_ast::BinaryOp::BitwiseAnd,
        qjs_ast::BinaryOp::BitwiseXor,
        qjs_ast::BinaryOp::BitwiseOr,
    ];
    let inputs = [
        (f64::NAN, 31.5),
        (f64::INFINITY, -1.0),
        (f64::NEG_INFINITY, 4_294_967_296.0),
        (-0.0, 1.0),
        (-1.5, 32.0),
    ];
    for (left, right) in inputs {
        for op in operations {
            let actual = number_binary(left, op, right).expect("number operation should be valid");
            let Some(Value::Number(expected)) = fast_number_binary_numbers(left, op, right) else {
                panic!("number fast path should return a Number");
            };
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "mismatch for {op:?} with {left:?} and {right:?}"
            );
        }
    }
}
