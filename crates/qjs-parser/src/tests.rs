use qjs_ast::{
    AssignmentOp, AssignmentTarget, BinaryOp, Expr, ForInLeft, ForInit, MemberProperty, Stmt,
    UnaryOp, UpdateOp, VarKind,
};

use super::parse_script;

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
fn parses_variable_declaration() {
    let script = parse_script("let answer = 40 + 2, missing;").expect("source should parse");
    let [
        Stmt::VarDecl {
            kind, declarations, ..
        },
    ] = script.body.as_slice()
    else {
        panic!("expected one variable declaration");
    };
    assert_eq!(*kind, VarKind::Let);
    assert_eq!(declarations.len(), 2);
    assert_eq!(declarations[0].name, "answer");
    assert!(matches!(
        declarations[0].init,
        Some(Expr::Binary {
            op: BinaryOp::Add,
            ..
        })
    ));
    assert_eq!(declarations[1].name, "missing");
    assert!(declarations[1].init.is_none());
}

#[test]
fn rejects_const_without_initializer() {
    let error = parse_script("const answer;").expect_err("const should require initializer");
    assert_eq!(error.message, "const declarations require an initializer");
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
fn parses_if_else_statement() {
    let script =
        parse_script("if (true) { let x = 1; } else { let x = 2; }").expect("source should parse");
    let [
        Stmt::If {
            consequent,
            alternate,
            ..
        },
    ] = script.body.as_slice()
    else {
        panic!("expected one if statement");
    };
    assert!(matches!(consequent.as_ref(), Stmt::Block { .. }));
    assert!(matches!(alternate.as_deref(), Some(Stmt::Block { .. })));
}

#[test]
fn parses_while_statement() {
    let script = parse_script("while (x < 3) { x = x + 1; }").expect("source should parse");
    let [Stmt::While { body, .. }] = script.body.as_slice() else {
        panic!("expected one while statement");
    };
    assert!(matches!(body.as_ref(), Stmt::Block { .. }));
}

#[test]
fn parses_do_while_statement() {
    let script = parse_script("do { x++; } while (x < 3);").expect("source should parse");
    let [Stmt::DoWhile { body, .. }] = script.body.as_slice() else {
        panic!("expected one do-while statement");
    };
    assert!(matches!(body.as_ref(), Stmt::Block { .. }));
}

#[test]
fn parses_for_statement() {
    let script =
        parse_script("for (var i = 0; i < 3; i = i + 1) { i; }").expect("source should parse");
    let [
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        },
    ] = script.body.as_slice()
    else {
        panic!("expected one for statement");
    };
    assert!(matches!(
        init,
        Some(ForInit::VarDecl { declarations, .. })
            if declarations.len() == 1 && declarations[0].name == "i"
    ));
    assert!(matches!(
        test,
        Some(Expr::Binary {
            op: BinaryOp::Lt,
            ..
        })
    ));
    assert!(matches!(update, Some(Expr::Assignment { .. })));
    assert!(matches!(body.as_ref(), Stmt::Block { .. }));
}

#[test]
fn parses_for_in_statement() {
    let script = parse_script("for (var key in object) { key; }").expect("source should parse");
    let [Stmt::ForIn { left, body, .. }] = script.body.as_slice() else {
        panic!("expected one for-in statement");
    };
    assert!(matches!(
        left,
        ForInLeft::VarDecl { name, .. } if name == "key"
    ));
    assert!(matches!(body.as_ref(), Stmt::Block { .. }));

    let script = parse_script("for (key in object) key;").expect("source should parse");
    let [Stmt::ForIn { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-in statement");
    };
    assert!(matches!(left, ForInLeft::Target(_)));
}

#[test]
fn parses_switch_statement() {
    let script =
        parse_script("switch (x) { case 1: x += 1; break; default: x = 0; case 2: x += 2; }")
            .expect("source should parse");
    let [Stmt::Switch { cases, .. }] = script.body.as_slice() else {
        panic!("expected one switch statement");
    };
    assert_eq!(cases.len(), 3);
    assert!(cases[0].test.is_some());
    assert!(cases[1].test.is_none());
    assert!(cases[2].test.is_some());
    assert_eq!(cases[0].consequent.len(), 2);
}

#[test]
fn parses_break_and_continue_statements() {
    let script = parse_script("while (true) { continue; break; }").expect("source should parse");
    let [Stmt::While { body, .. }] = script.body.as_slice() else {
        panic!("expected one while statement");
    };
    let Stmt::Block { body, .. } = body.as_ref() else {
        panic!("expected block body");
    };
    assert!(matches!(
        body.as_slice(),
        [Stmt::Continue { .. }, Stmt::Break { .. }]
    ));
}

#[test]
fn parses_throw_statement_with_argument() {
    let script = parse_script("if (false) { throw 'fail'; }").expect("source should parse");
    let [Stmt::If { consequent, .. }] = script.body.as_slice() else {
        panic!("expected one if statement");
    };
    let Stmt::Block { body, .. } = consequent.as_ref() else {
        panic!("expected block consequent");
    };
    assert!(matches!(
        body.as_slice(),
        [Stmt::Throw {
            argument: Some(_),
            ..
        }]
    ));
}

#[test]
fn parses_try_catch_finally_statement() {
    let script = parse_script("try { throw 'x'; } catch (error) { error; } finally { debugger; }")
        .expect("source should parse");
    let [
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        },
    ] = script.body.as_slice()
    else {
        panic!("expected one try statement");
    };
    assert!(matches!(block.as_slice(), [Stmt::Throw { .. }]));
    let handler = handler.as_ref().expect("expected catch clause");
    assert_eq!(handler.param.as_deref(), Some("error"));
    assert_eq!(handler.body.len(), 1);
    assert!(matches!(
        finalizer.as_deref(),
        Some([Stmt::Debugger { .. }])
    ));

    let script = parse_script("try { 1; } finally { 2; }").expect("source should parse");
    assert!(matches!(
        script.body.as_slice(),
        [Stmt::Try {
            handler: None,
            finalizer: Some(_),
            ..
        }]
    ));
}

#[test]
fn parses_debugger_statement() {
    let script = parse_script("debugger; 1;").expect("source should parse");
    assert!(matches!(
        script.body.as_slice(),
        [Stmt::Debugger { .. }, Stmt::Expr(_)]
    ));
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
fn parses_function_declaration_and_call() {
    let script = parse_script("function add(a, b) { return a + b; } add(1, 2);").expect("source");
    let [
        Stmt::FunctionDecl { name, params, .. },
        Stmt::Expr(Expr::Call { arguments, .. }),
    ] = script.body.as_slice()
    else {
        panic!("expected function declaration followed by call");
    };
    assert_eq!(name, "add");
    assert_eq!(params, &["a", "b"]);
    assert_eq!(arguments.len(), 2);

    let script = parse_script("let f = function named(value) { return value; }; f(1);")
        .expect("source should parse");
    let [
        Stmt::VarDecl { declarations, .. },
        Stmt::Expr(Expr::Call { .. }),
    ] = script.body.as_slice()
    else {
        panic!("expected function expression assignment followed by call");
    };
    let Some(Expr::Function { name, params, .. }) = &declarations[0].init else {
        panic!("expected function expression initializer");
    };
    assert_eq!(name.as_deref(), Some("named"));
    assert_eq!(params, &["value"]);
}

#[test]
fn parses_new_expression() {
    let script = parse_script("new Point(1, 2);").expect("source should parse");
    let [
        Stmt::Expr(Expr::New {
            callee, arguments, ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected one new expression statement");
    };
    assert!(matches!(callee.as_ref(), Expr::Identifier { name, .. } if name == "Point"));
    assert_eq!(arguments.len(), 2);
}

#[test]
fn parses_array_literal() {
    let script = parse_script("[1, 2 + 3,];").expect("source should parse");
    let [Stmt::Expr(Expr::Array { elements, .. })] = script.body.as_slice() else {
        panic!("expected one array expression");
    };
    assert_eq!(elements.len(), 2);
}

#[test]
fn parses_object_literal_and_member_assignment() {
    let script = parse_script("let object = { answer: 42, 'name': 7, }; object.answer = 43;")
        .expect("source should parse");
    let [
        Stmt::VarDecl { declarations, .. },
        Stmt::Expr(Expr::Assignment { target, .. }),
    ] = script.body.as_slice()
    else {
        panic!("expected object declaration followed by member assignment");
    };
    let Some(Expr::Object { properties, .. }) = &declarations[0].init else {
        panic!("expected object initializer");
    };
    assert_eq!(properties.len(), 2);
    assert_eq!(properties[0].key, "answer");
    assert_eq!(properties[1].key, "name");
    assert!(matches!(target, AssignmentTarget::Member { .. }));

    let script = parse_script("({ true: 1, false: 2, null: 3 });").expect("source should parse");
    let [Stmt::Expr(Expr::Object { properties, .. })] = script.body.as_slice() else {
        panic!("expected one object expression");
    };
    assert_eq!(properties[0].key, "true");
    assert_eq!(properties[1].key, "false");
    assert_eq!(properties[2].key, "null");
}

#[test]
fn parses_member_access() {
    let script = parse_script("items[0].length;").expect("source should parse");
    let [
        Stmt::Expr(Expr::Member {
            object, property, ..
        }),
    ] = script.body.as_slice()
    else {
        panic!("expected member expression");
    };
    assert_eq!(property, &MemberProperty::Named("length".to_owned()));
    assert!(matches!(object.as_ref(), Expr::Member { .. }));
}
