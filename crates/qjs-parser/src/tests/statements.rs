use qjs_ast::{
    BinaryOp, BindingPattern, Expr, ForInLeft, ForInit, ObjectBindingPropertyKey, Stmt, VarKind,
};

use crate::parse_script;

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
    assert_eq!(declarations[0].binding.names(), vec!["answer"]);
    assert!(matches!(
        declarations[0].init,
        Some(Expr::Binary {
            op: BinaryOp::Add,
            ..
        })
    ));
    assert_eq!(declarations[1].binding.names(), vec!["missing"]);
    assert!(declarations[1].init.is_none());
}

#[test]
fn parses_variable_declaration_binding_patterns() {
    let script = parse_script("const [first = 1, , third] = xs, {value, key: renamed = 2} = obj;")
        .expect("source should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected one variable declaration");
    };

    assert_eq!(declarations[0].binding.names(), vec!["first", "third"]);
    assert!(matches!(
        declarations[0].binding,
        BindingPattern::Array { .. }
    ));
    assert_eq!(declarations[1].binding.names(), vec!["value", "renamed"]);
    assert!(matches!(
        declarations[1].binding,
        BindingPattern::Object { .. }
    ));
}

#[test]
fn parses_computed_object_binding_property_names() {
    let script = parse_script("let { [key()]: value, plain } = obj;").expect("source should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected one variable declaration");
    };
    let BindingPattern::Object { properties, .. } = &declarations[0].binding else {
        panic!("expected object binding pattern");
    };

    assert!(matches!(
        &properties[0].key,
        ObjectBindingPropertyKey::Computed(Expr::Call { .. })
    ));
    assert_eq!(properties[1].key.as_literal(), Some("plain"));
    assert_eq!(declarations[0].binding.names(), vec!["value", "plain"]);
}

#[test]
fn rejects_const_without_initializer() {
    let error = parse_script("const answer;").expect_err("const should require initializer");
    assert_eq!(error.message, "const declarations require an initializer");
}

#[test]
fn rejects_destructuring_declaration_without_initializer() {
    let error =
        parse_script("var [answer];").expect_err("destructuring should require initializer");
    assert_eq!(
        error.message,
        "destructuring declarations require an initializer"
    );
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
            if declarations.len() == 1 && declarations[0].binding.names() == vec!["i"]
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
        ForInLeft::VarDecl {
            binding: BindingPattern::Identifier { name, .. },
            ..
        } if name == "key"
    ));
    assert!(matches!(body.as_ref(), Stmt::Block { .. }));

    let script = parse_script("for (key in object) key;").expect("source should parse");
    let [Stmt::ForIn { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-in statement");
    };
    assert!(matches!(left, ForInLeft::Target(_)));

    let script =
        parse_script("for (var key = init() in object) key;").expect("source should parse");
    let [Stmt::ForIn { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-in statement");
    };
    assert!(matches!(
        left,
        ForInLeft::VarDecl {
            binding: BindingPattern::Identifier { name, .. },
            init: Some(_),
            ..
        } if name == "key"
    ));
    assert!(parse_script("\"use strict\"; for (var key = 0 in object) key;").is_err());
    parse_script("\"use strict\"; for (var key = 0; key < 1; key = key + 1) key;")
        .expect("strict var for-loop initializers should parse");
}

#[test]
fn parses_for_of_statement() {
    let script =
        parse_script("for (const value of values) { value; }").expect("source should parse");
    let [Stmt::ForOf { left, body, .. }] = script.body.as_slice() else {
        panic!("expected one for-of statement");
    };
    assert!(matches!(
        left,
        ForInLeft::VarDecl {
            binding: BindingPattern::Identifier { name, .. },
            kind: VarKind::Const,
            ..
        } if name == "value"
    ));
    assert!(matches!(body.as_ref(), Stmt::Block { .. }));

    let script =
        parse_script("for (target.value of values) target.value;").expect("source should parse");
    let [Stmt::ForOf { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-of statement");
    };
    assert!(matches!(left, ForInLeft::Target(_)));

    let script = parse_script("for (var let of values) { total = total + let; }")
        .expect("source should parse");
    let [Stmt::ForOf { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-of statement");
    };
    assert!(matches!(
        left,
        ForInLeft::VarDecl {
            binding: BindingPattern::Identifier { name, .. },
            kind: VarKind::Var,
            ..
        } if name == "let"
    ));
    assert!(parse_script("for (let let of values) {}").is_err());
    assert!(parse_script("for (const let of values) {}").is_err());
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
    let script = parse_script("outer: while (true) { continue outer; break outer; }")
        .expect("source should parse");
    let [Stmt::Labelled { label, body, .. }] = script.body.as_slice() else {
        panic!("expected one labelled statement");
    };
    assert_eq!(label, "outer");
    let Stmt::While { body, .. } = body.as_ref() else {
        panic!("expected labelled while statement");
    };
    let Stmt::Block { body, .. } = body.as_ref() else {
        panic!("expected block body");
    };
    assert!(matches!(
        body.as_slice(),
        [
            Stmt::Continue {
                label: Some(continue_label),
                ..
            },
            Stmt::Break {
                label: Some(break_label),
                ..
            }
        ] if continue_label == "outer" && break_label == "outer"
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
    assert!(matches!(
        &handler.param,
        Some(BindingPattern::Identifier { name, .. }) if name == "error"
    ));
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

    let script = parse_script("try { throw {}; } catch ({ error, value, }) { value; }")
        .expect("source should parse");
    let [
        Stmt::Try {
            handler: Some(handler),
            ..
        },
    ] = script.body.as_slice()
    else {
        panic!("expected try statement with catch clause");
    };
    let Some(BindingPattern::Object { properties, .. }) = &handler.param else {
        panic!("expected object catch binding pattern");
    };
    assert_eq!(
        properties
            .iter()
            .map(|property| property.key.as_literal().expect("literal key"))
            .collect::<Vec<_>>(),
        ["error", "value"]
    );
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
fn parses_destructuring_loop_heads_and_catch_patterns() {
    let script = parse_script("for (const [a, , b = 1] of pairs) a;").expect("source should parse");
    let [Stmt::ForOf { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-of statement");
    };
    assert!(matches!(
        left,
        ForInLeft::VarDecl {
            kind: VarKind::Const,
            binding: BindingPattern::Array { .. },
            init: None,
            ..
        }
    ));

    let script = parse_script("for (var {key, value} in store) key;").expect("source should parse");
    let [Stmt::ForIn { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-in statement");
    };
    assert!(matches!(
        left,
        ForInLeft::VarDecl {
            kind: VarKind::Var,
            binding: BindingPattern::Object { .. },
            ..
        }
    ));

    let script = parse_script("for ([a, b] of pairs) a;").expect("source should parse");
    let [Stmt::ForOf { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-of statement");
    };
    assert!(matches!(
        left,
        ForInLeft::Target(qjs_ast::AssignmentTarget::ArrayPattern { .. })
    ));

    let script = parse_script("for ({a} in keys) a;").expect("source should parse");
    let [Stmt::ForIn { left, .. }] = script.body.as_slice() else {
        panic!("expected one for-in statement");
    };
    assert!(matches!(
        left,
        ForInLeft::Target(qjs_ast::AssignmentTarget::ObjectPattern { .. })
    ));

    let script = parse_script("try {} catch ([a, {b = 1}]) { a; }").expect("source should parse");
    let [Stmt::Try { handler, .. }] = script.body.as_slice() else {
        panic!("expected one try statement");
    };
    let handler = handler.as_ref().expect("expected catch clause");
    assert!(matches!(handler.param, Some(BindingPattern::Array { .. })));

    // Pattern heads without `of`/`in` still parse as ordinary for loops.
    parse_script("for (let [a] = [1]; a < 2; a += 1) a;").expect("source should parse");
}

#[test]
fn parses_with_statement() {
    let script = parse_script("with (obj) body;").expect("source should parse");
    let [Stmt::With { object, body, .. }] = script.body.as_slice() else {
        panic!("expected one with statement");
    };
    assert!(matches!(object, Expr::Identifier { name, .. } if name == "obj"));
    assert!(matches!(body.as_ref(), Stmt::Expr(Expr::Identifier { .. })));

    // `with` remains usable as a property name (it is a reserved word only as a
    // statement keyword).
    parse_script("array.with(0, 9);").expect("source should parse");

    // `with` is a SyntaxError in strict-mode code.
    assert!(parse_script("'use strict'; with (obj) {}").is_err());
}
