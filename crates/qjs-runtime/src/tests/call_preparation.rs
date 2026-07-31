//! Callable-kind dispatch, now that ordinary bodies route through a prepared
//! call.
//!
//! `call_function` decides what kind of callable it has before it prepares
//! anything, and only the ordinary synchronous shape becomes a
//! `PreparedBytecodeCall`. This table pins that every other kind still reaches
//! its own path and produces the same value or error, because the separation
//! is only worth having if it did not quietly move one of them.

use crate::{Value, eval};

fn value_of(source: &str) -> Value {
    eval(source).expect("source must evaluate")
}

fn error_of(source: &str) -> String {
    eval(source).expect_err("source must fail").message
}

#[test]
fn every_callable_kind_still_produces_its_own_result() {
    let cases: &[(&str, &str, Value)] = &[
        (
            "ordinary",
            "function f(a) { return a + 1; } f(1);",
            Value::Number(2.0),
        ),
        (
            "ordinary with default parameter",
            "function f(a, b = 10) { return a + b; } f(1);",
            Value::Number(11.0),
        ),
        (
            "ordinary with rest parameter",
            "function f(...rest) { return rest.length; } f(1, 2, 3);",
            Value::Number(3.0),
        ),
        (
            "prototype method reading this",
            "function C(v) { this.v = v; }
             C.prototype.get = function (a) { return this.v + a; };
             new C(1).get(2);",
            Value::Number(3.0),
        ),
        (
            "capturing closure",
            "function make(step) { return function (a) { return a + step; }; }
             make(5)(1);",
            Value::Number(6.0),
        ),
        (
            "named function expression recursion",
            "var f = function fact(n) { return n <= 1 ? 1 : n * fact(n - 1); };
             f(5);",
            Value::Number(120.0),
        ),
        ("native", "Math.max(1, 2, 3);", Value::Number(3.0)),
        (
            "bound",
            "function f(a, b) { return a + b; } f.bind(null, 1)(2);",
            Value::Number(3.0),
        ),
        (
            "proxy apply",
            "new Proxy(function (a) { return a + 1; }, {})(1);",
            Value::Number(2.0),
        ),
        (
            "base class construction",
            "class Base { constructor(v) { this.v = v; } } new Base(7).v;",
            Value::Number(7.0),
        ),
        (
            "derived construction returning this",
            "class Base { constructor(v) { this.v = v; } }
             class Derived extends Base { constructor(v) { super(v + 1); } }
             new Derived(1).v;",
            Value::Number(2.0),
        ),
        (
            "class field initializer",
            "class C { field = 4; } new C().field;",
            Value::Number(4.0),
        ),
        (
            "generator resumed twice",
            "function* g() { yield 1; yield 2; }
             var it = g();
             it.next().value + it.next().value;",
            Value::Number(3.0),
        ),
        (
            "direct-eval-capable body",
            "function f(a) { var local = a; eval('local = local + 1;'); return local; }
             f(1);",
            Value::Number(2.0),
        ),
        (
            "with-statement body",
            "function f(o) { with (o) { return x + 1; } } f({ x: 1 });",
            Value::Number(2.0),
        ),
        (
            "arguments object",
            "function f() { return arguments.length + arguments[0]; } f(5, 6);",
            Value::Number(7.0),
        ),
    ];
    for (name, source, expected) in cases {
        assert_eq!(value_of(source), *expected, "{name}");
    }
}

#[test]
fn an_async_function_still_returns_a_promise_rather_than_its_body_value() {
    let value = value_of("async function f() { return 1; } typeof f().then;");
    assert_eq!(value, Value::string_from_utf8("function"));
}

#[test]
fn dispatch_errors_are_unchanged_by_the_preparation_split() {
    let cases: &[(&str, &str, &str)] = &[
        ("not callable", "var x = 1; x();", "not callable"),
        (
            "class constructor called without new",
            "class C {} C();",
            "class constructor cannot be invoked without 'new'",
        ),
        (
            "derived constructor finishing without super",
            "class Base {} class Derived extends Base { constructor() {} } new Derived();",
            "ReferenceError",
        ),
    ];
    for (name, source, expected) in cases {
        let message = error_of(source);
        assert!(
            message.contains(expected),
            "{name}: expected {expected:?} in {message:?}"
        );
    }
}

#[test]
fn a_named_function_expression_restores_the_caller_binding_it_shadowed() {
    // The completion policy carries this restoration. If it were dropped, the
    // caller's `helper` would still be the callee after the call.
    let value = value_of(
        "var helper = 'caller';
         var run = function helper(depth) {
             return depth > 0 ? helper(depth - 1) : typeof helper;
         };
         run(2) + ':' + helper;",
    );
    assert_eq!(value, Value::string_from_utf8("function:caller"));
}
