//! Focused regression coverage for virtual constant forwarding.

use super::forward_read_only_virtual_constants;
use crate::{
    Value,
    bytecode::ir::{Bytecode, Op},
};
use qjs_ast::BinaryOp;

#[test]
fn lowering_preserves_nan_infinity_and_negative_zero_constant_bits() {
    let nan = f64::from_bits(0x7ff8_0000_0000_1234);
    let negative_zero = -0.0_f64;
    let infinity = f64::INFINITY;
    let bytecode = Bytecode::new(
        vec![
            Value::Number(nan),
            Value::Number(negative_zero),
            Value::Number(infinity),
            Value::Number(1.0),
        ],
        Vec::new(),
        Vec::new(),
    );
    let mut code = vec![
        Op::InitVirtualConstants {
            slot: 4,
            constants: vec![0, 1, 2],
            local: None,
            skip: 0,
        },
        Op::LoadVirtualValue {
            slot: 4,
            discard: 0,
        },
        Op::LoadVirtualValue {
            slot: 5,
            discard: 0,
        },
        Op::LoadVirtualValue {
            slot: 6,
            discard: 0,
        },
    ];
    forward_read_only_virtual_constants(&bytecode, &mut code);
    assert!(matches!(
        code[0],
        Op::InitVirtualObject {
            slot: 4,
            count: 0,
            ..
        }
    ));
    assert!(matches!(code[1], Op::LoadConst(0)));
    assert!(matches!(code[2], Op::LoadConst(1)));
    assert!(matches!(code[3], Op::LoadConst(2)));
    let Value::Number(forwarded_nan) = bytecode.constants[0] else {
        panic!("expected Number constant");
    };
    let Value::Number(forwarded_zero) = bytecode.constants[1] else {
        panic!("expected Number constant");
    };
    let Value::Number(forwarded_infinity) = bytecode.constants[2] else {
        panic!("expected Number constant");
    };
    assert_eq!(forwarded_nan.to_bits(), nan.to_bits());
    assert_eq!(forwarded_zero.to_bits(), negative_zero.to_bits());
    assert_eq!(forwarded_infinity.to_bits(), infinity.to_bits());

    // The numeric superinstruction must preserve the same IEEE result
    // that the runtime's existing fast-number path would have produced.
    let mut numeric_binary = vec![
        Op::InitVirtualConstants {
            slot: 8,
            constants: vec![1, 3],
            local: None,
            skip: 0,
        },
        Op::LoadVirtualBinary {
            left: 8,
            right: 9,
            op: BinaryOp::Mul,
            skip: 2,
        },
    ];
    forward_read_only_virtual_constants(&bytecode, &mut numeric_binary);
    assert!(matches!(
        numeric_binary[0],
        Op::InitVirtualObject {
            slot: 8,
            count: 0,
            ..
        }
    ));
    let Op::LoadVirtualNumber { value, skip } = numeric_binary[1] else {
        panic!("expected a folded numeric virtual binary");
    };
    assert_eq!(value.to_bits(), negative_zero.to_bits());
    assert_eq!(skip, 2);

    // Relational operators return booleans. Leave them on the existing
    // virtual-binary path instead of broadening this number-only opcode.
    let mut comparison = vec![
        Op::InitVirtualConstants {
            slot: 12,
            constants: vec![1, 2],
            local: None,
            skip: 0,
        },
        Op::LoadVirtualBinary {
            left: 12,
            right: 13,
            op: BinaryOp::Lt,
            skip: 2,
        },
    ];
    forward_read_only_virtual_constants(&bytecode, &mut comparison);
    assert!(matches!(comparison[0], Op::InitVirtualConstants { .. }));
    assert!(matches!(comparison[1], Op::LoadVirtualBinary { .. }));
}
