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
    assert_eq!(
        eval("var globalVar = 42; this.hasOwnProperty('globalVar') && globalVar === 42;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "var globalVarAttributes; let d = Object.getOwnPropertyDescriptor(this, 'globalVarAttributes'); d.enumerable && d.writable && !d.configurable;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("var nonDeletable; delete this.nonDeletable; this.hasOwnProperty('nonDeletable');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("this.reflectedVar = 99; var reflectedVar; reflectedVar;"),
        Ok(Value::Number(99.0))
    );
    assert_eq!(
        eval(
            "function f() { var localOnly = 1; return localOnly; } f(); this.hasOwnProperty('localOnly');"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "Object.defineProperty(Object.prototype, 'inheritedVarName', { value: 1001, writable: false, configurable: true }); var inheritedVarName = 1002; let result = this.hasOwnProperty('inheritedVarName') && inheritedVarName === 1002; delete Object.prototype.inheritedVarName; result;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("eval('7; var x;');"), Ok(Value::Number(7.0)));
    assert_eq!(eval("eval('9; let x = 10;');"), Ok(Value::Number(9.0)));
    assert_eq!(eval("eval('11; const x = 12;');"), Ok(Value::Number(11.0)));
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
    assert_eq!(
        eval("eval('1; do { 2; if (true) { 3; break; } 4; } while (false)');"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("eval('5; do { 6; if (true) { break; } 7; } while (false)');"),
        Ok(Value::Undefined)
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
fn evaluates_with_statements() {
    assert_eq!(
        eval(
            "function f() { var x = 0; var scope = { x: 2 }; with (scope) { x *= 3; } return scope.x === 6 && x === 0; } f();"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function f() { var x = 0; var scope = { get x() { delete this.x; return 2; } }; with (scope) { x *= 3; } return scope.x === 6 && x === 0; } f();"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "var outerScope = { x: 0 }; var innerScope = { get x() { delete this.x; return 5; } }; with (outerScope) { with (innerScope) { x &= 3; } } innerScope.x === 1 && outerScope.x === 0;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("var scope = { x: 1 }; with (scope) { var x = 2; } scope.x;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "var scope = { x: 1 }; with (scope) { var x = delete scope.x; } scope.x === true && x === undefined;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("var scope = {}; with (scope) { var x = 2; } scope.x === undefined && x === 2;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("var scope = { x: 1 }; with (scope) { delete x; } scope.x;"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "let caught = false; try { eval(\"'use strict'; with ({}) {}\"); } catch (error) { caught = error instanceof SyntaxError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { Function(\"'use strict'; with ({}) {}\"); } catch (error) { caught = error instanceof SyntaxError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "this.x = 1; let scope = { x: 2, value: 'boom' }; let result; try { with (scope) { x = 3; throw value; } } catch (error) { result = x; } result + ':' + x + ':' + scope.x;"
        ),
        Ok(Value::String("1:1:3".to_owned()))
    );
    assert_eq!(
        eval(
            "let scope = {}; let result = 0; do { with (scope) { result = 4; break; } result = 5; } while (false); result;"
        ),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("eval('1; do { 2; with({}) { 3; break; } 4; } while (false);');"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("eval('5; do { 6; with({}) { break; } 7; } while (false);');"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("eval('8; do { 9; with({}) { 10; continue; } 11; } while (false)');"),
        Ok(Value::Number(10.0))
    );
    assert_eq!(
        eval("eval('12; do { 13; with({}) { continue; } 14; } while (false)');"),
        Ok(Value::Undefined)
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
    assert_eq!(
        eval(
            "var value; for (let [x] = [23]; ; ) { value = x; break; } typeof x === 'undefined' && value === 23;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "var i = 0; var counter = 0; for (async of => {}; i < 10; ++i) { ++counter; } counter;"
        ),
        Ok(Value::Number(10.0))
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
            "let proto = {}; Object.defineProperty(proto, 'inherited', { value: 1, enumerable: true }); let object = Object.create(proto); object.own = 2; let seen = ''; for (let key in object) { seen = seen + key + ':'; } seen;"
        ),
        Ok(Value::String("own:inherited:".to_owned()))
    );
    assert_eq!(
        eval(
            "function f() {} Object.defineProperty(Function.prototype, 'inherited', { value: 1, enumerable: true, configurable: true }); let seen = false; for (let key in f.bind({})) { if (key === 'inherited') seen = true; } delete Function.prototype.inherited; seen;"
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
    assert_eq!(
        eval("eval('for(;;) { 1; break; }');"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("eval('var c = 0; for (; c < 3; c++) { if (c === 2) break; else c; }');"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "let out = ''; outer: for (let i = 0; i < 3; i++) { inner: for (let j = 0; j < 3; j++) { if (i * j >= 2) break outer; out += '' + i + j; } } out;"
        ),
        Ok(Value::String("0001021011".to_owned()))
    );
    assert_eq!(
        eval("let i = 0; woohoo: { while (true) { i++; if (i === 10) break woohoo; } i = 99; } i;"),
        Ok(Value::Number(10.0))
    );
    assert_eq!(
        eval(
            "var count = 0; for (let x = 0; x < 10;) { x++; count++; { let x = 'hello'; continue; } } count;"
        ),
        Ok(Value::Number(10.0))
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
    assert_eq!(
        eval("eval('5; do { switch (\"a\") { case \"a\": { 6; continue; } } } while (false)');"),
        Ok(Value::Number(6.0))
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
    assert_eq!(
        eval(
            "let count = 0; let finalized = 0; do { try { count++; continue; } finally { finalized++; } } while (count < 2); finalized;"
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("do { -99; try { 39; } finally { 42; break; } } while (false);"),
        Ok(Value::Number(42.0))
    );
    let error =
        eval("try { throw 'try'; } finally { throw 'finally'; }").expect_err("throw should fail");
    assert_eq!(error.message, "throw statement executed: finally");
    assert_eq!(
        eval(
            "let caught = ''; let finalized = false; try { try { throw 'inner'; } finally { throw 'finally'; } } catch (error) { caught = error; } finally { finalized = true; } caught + ':' + finalized;"
        ),
        Ok(Value::String("finally:true".to_owned()))
    );
    assert_eq!(
        eval("let error = 'outer'; try { throw 'inner'; } catch (error) { error; } error;"),
        Ok(Value::String("outer".to_owned()))
    );
    assert_eq!(
        eval(
            "var probe, x; try { throw 'inside'; } catch (x) { probe = function() { return x; }; } x = 'outside'; probe() + ':' + x;"
        ),
        Ok(Value::String("inside:outside".to_owned()))
    );
    assert_eq!(
        eval(
            "var probe, x = 'outside'; try { throw ['inside']; } catch ([x, _ = probe = function() { return x; }]) {} probe() + ':' + x;"
        ),
        Ok(Value::String("inside:outside".to_owned()))
    );
    assert_eq!(
        eval("var x = 'outside'; try { throw ['inside']; } catch ([x]) {} x;"),
        Ok(Value::String("outside".to_owned()))
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
