use crate::{Value, eval};

#[test]
fn evaluates_variable_declarations() {
    assert_eq!(
        eval("let x = 2; const y = 3; x * y;"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(eval("var missing; missing;"), Ok(Value::Undefined));
    assert_eq!(eval("x; var x;"), Ok(Value::Undefined));
    assert_eq!(eval("x; var x = 1; x;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("if (false) { var x = 1; } x;"), Ok(Value::Undefined));
    assert_eq!(
        eval("function f() { return x; var x = 2; } f();"),
        Ok(Value::Undefined)
    );
    assert!(eval("x; let x;").is_err());
    assert_eq!(
        eval("var x = 1, y = 2, missing; x + y;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_if_else_statements() {
    assert_eq!(
        eval("let x = 1; if (x > 0) { x = 7; } else { x = 3; } x;"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("let x = 1; if (x < 0) x = 7; else x = 3; x;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_while_statements() {
    assert_eq!(
        eval("let x = 0; while (x < 3) { x = x + 1; } x;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_do_while_statements() {
    assert_eq!(
        eval("let x = 0; do { x = x + 1; } while (false); x;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let x = 0; do { x++; } while (x < 3); x;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("let x = 0; do { x++; if (x === 2) continue; } while (x < 3); x;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_for_statements() {
    assert_eq!(
        eval("let sum = 0; for (var i = 0; i < 4; i = i + 1) { sum = sum + i; } sum;"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("let i = 0; for (; i < 3; ) i = i + 1; i;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_for_in_statements() {
    assert_eq!(
        eval("let count = 0; for (var key in { a: 1, b: 2 }) { count++; } count;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let total = 0; let item; let values = [1, 2, 3]; for (item in values) { total += values[item]; } total;"
        ),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("let count = 0; for (var key in null) { count++; } count;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let proto = {}; Object.defineProperty(proto, 'inherited', { value: 1, enumerable: true }); let object = Object.create(proto); object.own = 2; let seen = ''; for (var key in object) { seen = seen + key + ':'; } seen;"
        ),
        Ok(Value::String("own:inherited:".to_owned()))
    );
    assert_eq!(
        eval(
            "Object.defineProperty(Boolean.prototype, 'visible', { value: 1, enumerable: true, configurable: true }); let seen = false; for (var key in new Boolean()) { if (key === 'visible') seen = true; } delete Boolean.prototype.visible; seen;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_break_and_continue() {
    assert_eq!(
        eval("let i = 0; while (true) { i = i + 1; if (i === 3) break; } i;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "let sum = 0; for (var i = 0; i < 5; i = i + 1) { if (i === 2) continue; sum = sum + i; } sum;"
        ),
        Ok(Value::Number(8.0))
    );
}

#[test]
fn evaluates_switch_statements() {
    assert_eq!(
        eval(
            "let x = 2; let out = 0; switch (x) { case 1: out = 1; break; case 2: out = 2; break; default: out = 3; } out;"
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let x = 4; let out = 0; switch (x) { case 1: out = 1; break; default: out = 3; } out;"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "let x = 1; let out = 0; switch (x) { case 1: out += 1; case 2: out += 2; default: out += 4; } out;"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let x = '1'; let out = 0; switch (x) { case 1: out = 1; break; default: out = 2; } out;"
        ),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn evaluates_throw_statement_only_when_reached() {
    assert_eq!(eval("if (false) { throw; } 1;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("if (false) { throw 'no'; } 1;"),
        Ok(Value::Number(1.0))
    );
    let error = eval("throw;").expect_err("throw should fail evaluation");
    assert_eq!(error.message, "throw statement executed: undefined");
    let error = eval("throw 'expected';").expect_err("throw should fail evaluation");
    assert_eq!(error.message, "throw statement executed: expected");
    let error = eval("throw 42;").expect_err("throw should fail evaluation");
    assert_eq!(error.message, "throw statement executed: 42");
}

#[test]
fn evaluates_try_catch_finally_statements() {
    assert_eq!(
        eval("try { throw 'caught'; } catch (error) { error; }"),
        Ok(Value::String("caught".to_owned()))
    );
    assert_eq!(
        eval("let x = 1; try { throw 2; } catch (error) { x = error; } x;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("let x = 1; try { x += 1; } finally { x += 2; } x;"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("let x = 1; try { throw 1; } catch (error) { x += error; } finally { x += 2; } x;"),
        Ok(Value::Number(4.0))
    );
    let error =
        eval("try { throw 'try'; } finally { throw 'finally'; }").expect_err("throw should fail");
    assert_eq!(error.message, "throw statement executed: finally");
    assert_eq!(
        eval("let error = 'outer'; try { throw 'inner'; } catch (error) { error; } error;"),
        Ok(Value::String("outer".to_owned()))
    );
}

#[test]
fn evaluates_debugger_statement_as_noop() {
    assert_eq!(eval("debugger; 1;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("let x = 0; if (true) debugger; x = 2; x;"),
        Ok(Value::Number(2.0))
    );
}
