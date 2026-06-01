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
fn evaluates_delete_with_bytecode() {
    assert_bytecode_matches_ast("let object = { value: 1 }; delete object.value;");
    assert_bytecode_matches_ast("let object = { value: 1 }; delete object.value; object.value;");
    assert_bytecode_matches_ast(
        "let key = 'value'; let object = { value: 1 }; delete object[key]; object.value;",
    );
    assert_bytecode_matches_ast("let array = [1, 2]; delete array[0]; array[0];");
    assert_bytecode_matches_ast("let x = 1; delete x;");
    assert_bytecode_matches_ast("delete missing;");
    assert_eq!(eval_bytecode_source("delete 1;"), eval("delete 1;"));
}

#[test]
fn evaluates_branch_and_loop_bytecode_subset() {
    assert_bytecode_matches_ast("let x = 1; if (x > 0) { x = 7; } else { x = 3; } x;");
    assert_bytecode_matches_ast("if (false) { 1; }");
    assert_bytecode_matches_ast("let x = 0; while (x < 3) { x = x + 1; } x;");
    assert_bytecode_matches_ast("let x = 0; while (x < 3) { x = x + 1; x; }");
    assert_bytecode_matches_ast("let x = 0; while (true) { x++; if (x === 3) break; } x;");
    assert_bytecode_matches_ast(
        "let x = 0; let sum = 0; while (x < 5) { x++; if (x === 3) continue; sum = sum + x; } sum;",
    );
    assert_bytecode_matches_ast("let x = 0; do { x++; } while (x < 3); x;");
    assert_bytecode_matches_ast("let x = 0; do { x++; x; } while (x < 3);");
    assert_bytecode_matches_ast("let x = 0; do { x++; if (x < 3) continue; x; } while (x < 5);");
    assert_bytecode_matches_ast(
        "let sum = 0; for (var i = 0; i < 4; i = i + 1) { sum = sum + i; } sum;",
    );
    assert_bytecode_matches_ast("for (var i = 0; i < 3; i = i + 1) { i; }");
    assert_bytecode_matches_ast(
        "let sum = 0; for (var i = 0; i < 6; i = i + 1) { if (i === 2) continue; if (i === 5) break; sum = sum + i; } sum;",
    );
    assert_bytecode_matches_ast(
        "let x = 2; switch (x) { case 1: 'one'; break; case 2: 'two'; break; default: 'other'; }",
    );
    assert_bytecode_matches_ast(
        "let x = 3; switch (x) { case 1: 'one'; break; default: 'other'; }",
    );
    assert_bytecode_matches_ast("let x = 1; switch (x) { case 1: 'one'; case 2: 'two'; }");
    assert_bytecode_matches_ast(
        "let sum = 0; for (var i = 0; i < 4; i++) { switch (i) { case 1: continue; case 3: break; default: sum = sum + i; } sum = sum + 10; } sum;",
    );
    assert_bytecode_matches_ast(
        "function f(x) { switch (x) { case 1: return 'one'; default: return 'other'; } } f(1);",
    );
    assert_bytecode_matches_ast(
        "let out = ''; for (var key in { b: 2, a: 1 }) { out = out + key; } out;",
    );
    assert_bytecode_matches_ast(
        "let out = ''; let key; for (key in ['x', 'y']) { out = out + key; } out;",
    );
    assert_bytecode_matches_ast("let out = ''; for (var key in null) { out = out + key; } out;");
    assert_bytecode_matches_ast(
        "let out = ''; for (var key in { a: 1, b: 2, c: 3 }) { if (key === 'b') continue; out = out + key; if (key === 'c') break; } out;",
    );
    assert_bytecode_matches_ast("let holder = {}; for (holder.key in { z: 1 }) {} holder.key;");
    assert_bytecode_matches_ast(
        "function keys(object) { let out = ''; for (var key in object) { out = out + key; } return out; } keys({ b: 2, a: 1 });",
    );
    assert_bytecode_matches_ast("try { 1; } catch (error) { 2; }");
    assert_bytecode_matches_ast("try { throw 'hit'; } catch (error) { error; }");
    assert_bytecode_matches_ast("let x = 1; try { x = 2; } finally { x = x + 3; } x;");
    assert_bytecode_matches_ast(
        "let x = ''; try { throw 'a'; } catch (error) { x = x + error; } finally { x = x + 'f'; } x;",
    );
    assert_bytecode_matches_ast(
        "let x = ''; try { throw 'a'; } catch (error) { throw 'b'; } finally { x = x + 'f'; }",
    );
    assert_bytecode_matches_ast(
        "function f() { let x = 1; try { return x; } finally { x = 3; } } f();",
    );
    assert_bytecode_matches_ast(
        "function f() { try { throw 'a'; } catch (error) { return error; } finally { 2; } } f();",
    );
    assert_bytecode_matches_ast("function f() { try { return 1; } finally { return 2; } } f();");
    assert_bytecode_matches_ast(
        "function f() { try { throw 'a'; } finally { return 'final'; } } f();",
    );
}

#[test]
fn evaluates_objects_arrays_members_and_calls_with_bytecode() {
    assert_bytecode_matches_ast("let o = { count: 1 }; o.count;");
    assert_bytecode_matches_ast("let o = { count: 1 }; o.count = 3; o.count;");
    assert_bytecode_matches_ast("let key = 'count'; let o = { [key]: 2 }; o[key];");
    assert_bytecode_matches_ast("let values = [1, 2, 3]; values[1];");
    assert_bytecode_matches_ast("let values = [1, 2, 3]; values.length;");
    assert_bytecode_matches_ast("'abc'.length;");
    assert_bytecode_matches_ast("Math.max(1, 5, 3);");
    assert_bytecode_matches_ast("let values = [1]; values.push(2); values.length;");
    assert_bytecode_matches_ast("function f() { return 1; } f();");
    assert_bytecode_matches_ast("function add(a, b) { return a + b; } add(2, 3);");
    assert_bytecode_matches_ast("let base = 10; function add(x) { return base + x; } add(5);");
    assert_bytecode_matches_ast("let f = function() { return 2; }; f();");
    assert_bytecode_matches_ast("let f = function(x) { return x * 2; }; f(4);");
    assert_bytecode_matches_ast(
        "function C() { this.value = 4; } let instance = new C(); instance.value;",
    );
}

#[test]
fn reports_unsupported_bytecode_surface() {
    let error = eval_bytecode_source("break;").expect_err("top-level break must not compile");
    assert_eq!(error.message, "break outside loop");

    let error = eval_bytecode_source("continue;").expect_err("top-level continue must not compile");
    assert_eq!(error.message, "continue outside loop");
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
