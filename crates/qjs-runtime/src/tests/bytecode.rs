use crate::{Value, eval, eval_bytecode_source};

fn assert_bytecode_matches_ast(source: &str) {
    assert_eq!(eval_bytecode_source(source), eval(source), "{source}");
}

#[test]
fn evaluates_core_expressions_with_bytecode() {
    assert_bytecode_matches_ast("1 + 2 * 3;");
    assert_bytecode_matches_ast("'x' + 1;");
    assert_bytecode_matches_ast("true ? 1 : missing;");
    assert_bytecode_matches_ast("false || 5;");
    assert_bytecode_matches_ast("0 && missing;");
    assert_bytecode_matches_ast("null ?? 42;");
    assert_bytecode_matches_ast("typeof neverDeclared;");
}

#[test]
fn evaluates_slot_locals_with_bytecode() {
    assert_bytecode_matches_ast("let x = 2; const y = 3; x * y;");
    assert_bytecode_matches_ast("var x = 1, y = 2, missing; x + y;");
    assert_bytecode_matches_ast("x; var x = 1; x;");
    assert_bytecode_matches_ast("let x = 1; x += 2; x;");
    assert_bytecode_matches_ast("let x = 1; x++; x;");
    assert_bytecode_matches_ast("let x = 1; ++x;");
}

#[test]
fn evaluates_branch_and_loop_bytecode_subset() {
    assert_bytecode_matches_ast("let x = 1; if (x > 0) { x = 7; } else { x = 3; } x;");
    assert_bytecode_matches_ast("if (false) { 1; }");
    assert_bytecode_matches_ast("let x = 0; while (x < 3) { x = x + 1; } x;");
    assert_bytecode_matches_ast("let x = 0; while (x < 3) { x = x + 1; x; }");
    assert_bytecode_matches_ast("let x = 0; do { x++; } while (x < 3); x;");
    assert_bytecode_matches_ast("let x = 0; do { x++; x; } while (x < 3);");
    assert_bytecode_matches_ast(
        "let sum = 0; for (var i = 0; i < 4; i = i + 1) { sum = sum + i; } sum;",
    );
    assert_bytecode_matches_ast("for (var i = 0; i < 3; i = i + 1) { i; }");
}

#[test]
fn reports_unsupported_bytecode_surface() {
    let error = eval_bytecode_source("let o = { count: 1 }; o.count;")
        .expect_err("object literals are not in the first bytecode slice");
    assert!(error.message.contains("unsupported bytecode expression"));

    let error = eval_bytecode_source("function f() { return 1; } f();")
        .expect_err("functions are not in the first bytecode slice");
    assert!(error.message.contains("unsupported bytecode statement"));
}

#[test]
fn propagates_bytecode_throw_errors() {
    assert_eq!(
        eval_bytecode_source("if (false) { throw; } 1;"),
        Ok(Value::Number(1.0))
    );
    let error = eval_bytecode_source("throw 'expected';").expect_err("throw should fail");
    assert_eq!(error.message, "throw statement executed: expected");
}
