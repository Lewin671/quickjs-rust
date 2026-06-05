use qjs_ast::{AssignmentTarget, BinaryOp, Expr, ForInLeft, ForInit, Stmt, VarKind};

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
fn parses_sloppy_let_identifier_expression() {
    let script = parse_script("var let; let = 1;").expect("source should parse");
    let [
        Stmt::VarDecl { .. },
        Stmt::Expr(Expr::Assignment { target, .. }),
    ] = script.body.as_slice()
    else {
        panic!("expected var declaration followed by assignment expression");
    };
    assert!(matches!(
        target,
        qjs_ast::AssignmentTarget::Identifier { name, .. } if name == "let"
    ));
}

#[test]
fn rejects_const_without_initializer() {
    let error = parse_script("const answer;").expect_err("const should require initializer");
    assert_eq!(error.message, "const declarations require an initializer");
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
fn parses_with_statement() {
    let script = parse_script("with (scope) { x *= 3; }").expect("source should parse");
    let [Stmt::With { object, body, .. }] = script.body.as_slice() else {
        panic!("expected one with statement");
    };
    assert!(matches!(object, Expr::Identifier { name, .. } if name == "scope"));
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
fn parses_for_head_sloppy_let_identifier_expression() {
    let script = parse_script("for (let; ; ) break;").expect("source should parse");
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
        Some(ForInit::Expr(Expr::Identifier { name, .. })) if name == "let"
    ));
    assert!(test.is_none());
    assert!(update.is_none());
    assert!(matches!(body.as_ref(), Stmt::Break { .. }));
}

#[test]
fn parses_for_head_lexical_destructuring_initializer() {
    let script = parse_script("for (let [x] = [23]; ; ) break;").expect("source should parse");
    let [Stmt::For { init, .. }] = script.body.as_slice() else {
        panic!("expected one for statement");
    };
    assert!(matches!(
        init,
        Some(ForInit::Binding {
            kind: VarKind::Let,
            target: qjs_ast::AssignmentTarget::Array { .. },
            init: Expr::Array { .. },
            ..
        })
    ));
}

#[test]
fn parses_for_body_sloppy_let_before_newline_block() {
    let script = parse_script("for (; false; ) let\n{}").expect("source should parse");
    let [Stmt::For { body, .. }, Stmt::Block { .. }] = script.body.as_slice() else {
        panic!("expected for body expression followed by block statement");
    };
    assert!(matches!(
        body.as_ref(),
        Stmt::Expr(Expr::Identifier { name, .. }) if name == "let"
    ));
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

    let script = parse_script("while (true) { break\nlabel; continue\nlabel; }")
        .expect("source should parse");
    let [Stmt::While { body, .. }] = script.body.as_slice() else {
        panic!("expected one while statement");
    };
    let Stmt::Block { body, .. } = body.as_ref() else {
        panic!("expected block body");
    };
    assert!(matches!(
        body.as_slice(),
        [
            Stmt::Break { label: None, .. },
            Stmt::Expr(Expr::Identifier { name: break_name, .. }),
            Stmt::Continue { label: None, .. },
            Stmt::Expr(Expr::Identifier {
                name: continue_name,
                ..
            }),
        ] if break_name == "label" && continue_name == "label"
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
        handler.param,
        Some(AssignmentTarget::Identifier { ref name, .. }) if name == "error"
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

    let script = parse_script("try { throw [1]; } catch ([value = 2]) { value; }")
        .expect("source should parse");
    let [
        Stmt::Try {
            handler: Some(handler),
            ..
        },
    ] = script.body.as_slice()
    else {
        panic!("expected one try statement");
    };
    assert!(matches!(
        handler.param,
        Some(AssignmentTarget::Array { .. })
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
