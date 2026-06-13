use qjs_ast::{
    AssignmentOp, AssignmentTarget, AssignmentTargetPropertyKey, BinaryOp, Expr, Stmt, UnaryOp,
    UpdateOp,
};

use crate::parse_script;

#[test]
fn parses_binary_precedence() {
    let script = parse_script("1 + 2 * 3;").expect("source should parse");
    let [Stmt::Expr(Expr::Binary { op, .. })] = script.body.as_slice() else {
        panic!("expected one binary expression statement");
    };
    assert_eq!(*op, BinaryOp::Add);
}

#[test]
fn parses_comparison_before_equality() {
    let script = parse_script("1 + 2 >= 3 === true;").expect("source should parse");
    let [Stmt::Expr(Expr::Binary { op, left, .. })] = script.body.as_slice() else {
        panic!("expected one binary expression statement");
    };
    assert_eq!(*op, BinaryOp::StrictEq);
    let Expr::Binary { op: left_op, .. } = left.as_ref() else {
        panic!("expected comparison on left side");
    };
    assert_eq!(*left_op, BinaryOp::Ge);

    let script = parse_script("'x' in object;").expect("source should parse");
    let [Stmt::Expr(Expr::Binary { op, .. })] = script.body.as_slice() else {
        panic!("expected one binary expression statement");
    };
    assert_eq!(*op, BinaryOp::In);

    let script = parse_script("object instanceof Constructor;").expect("source should parse");
    let [Stmt::Expr(Expr::Binary { op, .. })] = script.body.as_slice() else {
        panic!("expected one binary expression statement");
    };
    assert_eq!(*op, BinaryOp::Instanceof);
}

#[test]
fn parses_shift_and_bitwise_precedence() {
    let script = parse_script("1 | 2 ^ 3 & 4 === 4;").expect("source should parse");
    let [Stmt::Expr(Expr::Binary { op, right, .. })] = script.body.as_slice() else {
        panic!("expected one binary expression statement");
    };
    assert_eq!(*op, BinaryOp::BitwiseOr);
    let Expr::Binary { op: right_op, .. } = right.as_ref() else {
        panic!("expected bitwise xor on right side");
    };
    assert_eq!(*right_op, BinaryOp::BitwiseXor);

    let script = parse_script("1 + 2 << 3 < 30;").expect("source should parse");
    let [Stmt::Expr(Expr::Binary { op, left, .. })] = script.body.as_slice() else {
        panic!("expected one comparison expression statement");
    };
    assert_eq!(*op, BinaryOp::Lt);
    assert!(matches!(
        left.as_ref(),
        Expr::Binary {
            op: BinaryOp::Shl,
            ..
        }
    ));
}

#[test]
fn parses_exponentiation_as_right_associative() {
    let script = parse_script("2 ** 3 ** 2;").expect("source should parse");
    let [Stmt::Expr(Expr::Binary { op, right, .. })] = script.body.as_slice() else {
        panic!("expected one binary expression statement");
    };
    assert_eq!(*op, BinaryOp::Pow);
    assert!(matches!(
        right.as_ref(),
        Expr::Binary {
            op: BinaryOp::Pow,
            ..
        }
    ));
}

#[test]
fn parses_logical_precedence() {
    let script = parse_script("true || false && false;").expect("source should parse");
    let [Stmt::Expr(Expr::Binary { op, right, .. })] = script.body.as_slice() else {
        panic!("expected one binary expression statement");
    };
    assert_eq!(*op, BinaryOp::LogicalOr);
    let Expr::Binary { op: right_op, .. } = right.as_ref() else {
        panic!("expected logical and on right side");
    };
    assert_eq!(*right_op, BinaryOp::LogicalAnd);
}

#[test]
fn parses_nullish_coalescing_expression() {
    let script = parse_script("null ?? 1 ?? 2;").expect("source should parse");
    let [Stmt::Expr(Expr::Binary { op, left, .. })] = script.body.as_slice() else {
        panic!("expected one binary expression statement");
    };
    assert_eq!(*op, BinaryOp::NullishCoalescing);
    assert!(matches!(
        left.as_ref(),
        Expr::Binary {
            op: BinaryOp::NullishCoalescing,
            ..
        }
    ));
}

#[test]
fn parses_conditional_expression_as_right_associative() {
    let script = parse_script("false ? 1 : true ? 2 : 3;").expect("source should parse");
    let [Stmt::Expr(Expr::Conditional { alternate, .. })] = script.body.as_slice() else {
        panic!("expected one conditional expression statement");
    };
    assert!(matches!(alternate.as_ref(), Expr::Conditional { .. }));
}

#[test]
fn parses_sequence_expression() {
    let script = parse_script("a = 1, b = 2, b;").expect("source should parse");
    let [Stmt::Expr(Expr::Sequence { expressions, .. })] = script.body.as_slice() else {
        panic!("expected one sequence expression statement");
    };
    assert_eq!(expressions.len(), 3);
    assert!(matches!(expressions[0], Expr::Assignment { .. }));
    assert!(matches!(expressions[1], Expr::Assignment { .. }));
}
#[test]
fn parses_assignment_as_right_associative() {
    let script = parse_script("a = b = 1;").expect("source should parse");
    let [
        Stmt::Expr(Expr::Assignment {
            target, op, value, ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected one assignment expression statement");
    };
    assert_eq!(*op, AssignmentOp::Assign);
    let AssignmentTarget::Identifier { name, .. } = target else {
        panic!("expected identifier assignment target");
    };
    assert_eq!(name, "a");
    let Expr::Assignment {
        target: inner_target,
        ..
    } = value.as_ref()
    else {
        panic!("expected nested assignment");
    };
    let AssignmentTarget::Identifier {
        name: inner_name, ..
    } = inner_target
    else {
        panic!("expected identifier assignment target");
    };
    assert_eq!(inner_name, "b");
}

#[test]
fn marks_parenthesized_identifier_assignment_targets() {
    let script = parse_script("(fn) = function() {};").expect("source should parse");
    let [
        Stmt::Expr(Expr::Assignment {
            target:
                AssignmentTarget::Identifier {
                    name,
                    parenthesized,
                    ..
                },
            ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected parenthesized identifier assignment target");
    };
    assert_eq!(name, "fn");
    assert!(*parenthesized);
}

#[test]
fn parses_update_and_compound_assignment() {
    let script = parse_script(
            "++i; i++; i += 2; obj.count--; a <<= b; c >>= d; e >>>= f; g &= h; i ^= j; k |= l; m &&= n; o ||= p; q ??= r;",
        )
            .expect("source should parse");
    let [
        Stmt::Expr(Expr::Update {
            op: UpdateOp::Increment,
            prefix: true,
            ..
        }),
        Stmt::Expr(Expr::Update {
            op: UpdateOp::Increment,
            prefix: false,
            ..
        }),
        Stmt::Expr(Expr::Assignment {
            op: AssignmentOp::AddAssign,
            ..
        }),
        Stmt::Expr(Expr::Update {
            op: UpdateOp::Decrement,
            prefix: false,
            ..
        }),
        Stmt::Expr(Expr::Assignment {
            op: AssignmentOp::ShlAssign,
            ..
        }),
        Stmt::Expr(Expr::Assignment {
            op: AssignmentOp::ShrAssign,
            ..
        }),
        Stmt::Expr(Expr::Assignment {
            op: AssignmentOp::UShrAssign,
            ..
        }),
        Stmt::Expr(Expr::Assignment {
            op: AssignmentOp::BitwiseAndAssign,
            ..
        }),
        Stmt::Expr(Expr::Assignment {
            op: AssignmentOp::BitwiseXorAssign,
            ..
        }),
        Stmt::Expr(Expr::Assignment {
            op: AssignmentOp::BitwiseOrAssign,
            ..
        }),
        Stmt::Expr(Expr::Assignment {
            op: AssignmentOp::LogicalAndAssign,
            ..
        }),
        Stmt::Expr(Expr::Assignment {
            op: AssignmentOp::LogicalOrAssign,
            ..
        }),
        Stmt::Expr(Expr::Assignment {
            op: AssignmentOp::NullishAssign,
            ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected update, compound assignment, and logical assignment statements");
    };
}

#[test]
fn rejects_invalid_assignment_target() {
    let error = parse_script("(1 + 2) = 3;").expect_err("assignment target should fail");
    assert_eq!(error.message, "invalid assignment target");
}
#[test]
fn parses_unary_before_multiplicative() {
    let script = parse_script("-1 * !false;").expect("source should parse");
    let [Stmt::Expr(Expr::Binary { left, right, .. })] = script.body.as_slice() else {
        panic!("expected one binary expression");
    };
    assert!(matches!(
        left.as_ref(),
        Expr::Unary {
            op: UnaryOp::Minus,
            ..
        }
    ));
    assert!(matches!(
        right.as_ref(),
        Expr::Unary {
            op: UnaryOp::Not,
            ..
        }
    ));

    let script = parse_script("typeof missing;").expect("source should parse");
    let [Stmt::Expr(Expr::Unary { op, .. })] = script.body.as_slice() else {
        panic!("expected one unary expression");
    };
    assert_eq!(*op, UnaryOp::Typeof);

    let script = parse_script("void sideEffect;").expect("source should parse");
    let [Stmt::Expr(Expr::Unary { op, .. })] = script.body.as_slice() else {
        panic!("expected one unary expression");
    };
    assert_eq!(*op, UnaryOp::Void);

    let script = parse_script("delete object.key;").expect("source should parse");
    let [Stmt::Expr(Expr::Unary { op, .. })] = script.body.as_slice() else {
        panic!("expected one unary expression");
    };
    assert_eq!(*op, UnaryOp::Delete);

    let script = parse_script("this.value;").expect("source should parse");
    let [Stmt::Expr(Expr::Member { object, .. })] = script.body.as_slice() else {
        panic!("expected one member expression");
    };
    assert!(matches!(object.as_ref(), Expr::This { .. }));
}

#[test]
fn parses_destructuring_assignment_patterns() {
    let script = parse_script("[a, , b = 1, ...rest] = list;").expect("source should parse");
    let [Stmt::Expr(Expr::Assignment { target, op, .. })] = script.body.as_slice() else {
        panic!("expected one assignment expression statement");
    };
    assert_eq!(*op, AssignmentOp::Assign);
    let AssignmentTarget::ArrayPattern { elements, rest, .. } = target else {
        panic!("expected array assignment pattern");
    };
    assert_eq!(elements.len(), 3);
    assert!(elements[1].is_none(), "expected elision element");
    let element = elements[2].as_ref().expect("expected defaulted element");
    assert!(element.default.is_some(), "expected default initializer");
    assert!(
        matches!(
            rest.as_deref(),
            Some(AssignmentTarget::Identifier { name, .. }) if name == "rest"
        ),
        "expected identifier rest target"
    );

    let script = parse_script("({key, renamed: out.field, nested: {inner}, ...rest} = source);")
        .expect("source should parse");
    let [Stmt::Expr(Expr::Assignment { target, .. })] = script.body.as_slice() else {
        panic!("expected one assignment expression statement");
    };
    let AssignmentTarget::ObjectPattern {
        properties, rest, ..
    } = target
    else {
        panic!("expected object assignment pattern");
    };
    assert_eq!(properties.len(), 3);
    assert!(matches!(
        &properties[0].target,
        AssignmentTarget::Identifier { name, .. } if name == "key"
    ));
    assert!(matches!(
        &properties[1].target,
        AssignmentTarget::Member { .. }
    ));
    assert!(matches!(
        &properties[2].target,
        AssignmentTarget::ObjectPattern { .. }
    ));
    assert!(rest.is_some(), "expected object rest target");
}

#[test]
fn parses_computed_object_assignment_property_names() {
    let script = parse_script("({ [key()]: value, plain } = obj);").expect("source should parse");
    let [Stmt::Expr(Expr::Assignment { target, .. })] = script.body.as_slice() else {
        panic!("expected assignment expression");
    };
    let AssignmentTarget::ObjectPattern { properties, .. } = target else {
        panic!("expected object assignment pattern");
    };

    assert!(matches!(
        &properties[0].key,
        AssignmentTargetPropertyKey::Computed(Expr::Call { .. })
    ));
    assert_eq!(properties[1].key.as_literal(), Some("plain"));
}

#[test]
fn keeps_member_expressions_on_literal_starts_out_of_patterns() {
    let script = parse_script("[ {}[key] ] = value;").expect("source should parse");
    let [Stmt::Expr(Expr::Assignment { target, .. })] = script.body.as_slice() else {
        panic!("expected one assignment expression statement");
    };
    let AssignmentTarget::ArrayPattern { elements, .. } = target else {
        panic!("expected array assignment pattern");
    };
    let element = elements[0].as_ref().expect("expected member element");
    assert!(matches!(element.target, AssignmentTarget::Member { .. }));
}

#[test]
fn rejects_invalid_destructuring_assignments() {
    assert!(parse_script("[a, ...rest = 1] = list;").is_err());
    assert!(parse_script("[1] = list;").is_err());
    assert!(parse_script("({a} = source) , [b] += list;").is_err());
}

#[test]
fn rejects_strict_assignment_to_eval_or_arguments() {
    // Simple and compound assignment to `eval`/`arguments` is an early
    // SyntaxError in strict mode.
    assert!(parse_script("'use strict'; eval = 1;").is_err());
    assert!(parse_script("'use strict'; arguments = 1;").is_err());
    assert!(parse_script("'use strict'; eval += 1;").is_err());
    assert!(parse_script("'use strict'; arguments *= 2;").is_err());
    // Sloppy mode permits both.
    assert!(parse_script("eval = 1;").is_ok());
    assert!(parse_script("arguments += 1;").is_ok());
}

#[test]
fn rejects_strict_yield_identifier_references() {
    assert!(parse_script("'use strict'; yield;").is_err());
    assert!(parse_script("'use strict'; yield = 1;").is_err());
    assert!(parse_script("yield = 1;").is_ok());
}

#[test]
fn rejects_strict_destructuring_target_eval_or_arguments() {
    // `eval`/`arguments` are not valid simple targets inside a destructuring
    // assignment under strict mode.
    assert!(parse_script("'use strict'; [arguments] = [];").is_err());
    assert!(parse_script("'use strict'; [eval] = [];").is_err());
    assert!(parse_script("'use strict'; ({ a: arguments } = {});").is_err());
    assert!(parse_script("'use strict'; ({ arguments } = {});").is_err());
    assert!(parse_script("'use strict'; ({ eval = 1 } = {});").is_err());
    // Sloppy mode permits them.
    assert!(parse_script("[arguments] = [];").is_ok());
    assert!(parse_script("({ eval } = {});").is_ok());
}

#[test]
fn rejects_reserved_object_assignment_shorthand_targets() {
    assert!(parse_script("function* g() { 0, { yield } = {}; }").is_err());
    assert!(parse_script("'use strict'; 0, { yield } = {};").is_err());
    assert!(parse_script("var x = { bre\\u0061k } = { break: 42 };").is_err());
    assert!(parse_script("var x = { cl\\u0061ss = 1 } = { class: 42 };").is_err());
    assert!(parse_script("var x = { \\u0069mport } = { import: 42 };").is_err());
    assert!(parse_script("var x = { enum } = { enum: 42 };").is_err());
    assert!(parse_script("'use strict'; var x = { l\\u0065t } = { let: 42 };").is_err());
    assert!(
        parse_script("'use strict'; var x = { \\u0069mplements } = { implements: 42 };").is_err()
    );
    parse_script("var x = { l\\u0065t } = { let: 42 };")
        .expect("escaped let remains an identifier target in sloppy mode");
    parse_script("var target; ({ break: target } = { break: 42 });")
        .expect("reserved words remain valid explicit property names");
}
