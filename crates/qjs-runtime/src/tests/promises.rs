use crate::{Value, eval};

fn assert_eval(source: &str, expected: Value) {
    assert_eq!(eval(source), Ok(expected));
}

#[test]
fn evaluates_promise_constructor_shell() {
    assert_eval("typeof Promise;", Value::String("function".to_owned()));
    assert_eval("Promise.length;", Value::Number(1.0));
    assert_eval(
        "new Promise(function(resolve) { resolve(1); }) instanceof Promise;",
        Value::Boolean(true),
    );
    assert_eval(
        "Object.prototype.toString.call(new Promise(function(resolve) { resolve(1); }));",
        Value::String("[object Promise]".to_owned()),
    );
    assert_eval(
        "var called = false; new Promise(function(resolve, reject) { called = typeof resolve + ':' + typeof reject; resolve(1); }); called;",
        Value::String("function:function".to_owned()),
    );
    assert!(eval("Promise(function() {});").is_err());
    assert!(eval("new Promise(1);").is_err());
}

#[test]
fn evaluates_promise_resolve_reject_shell() {
    assert_eval("Promise.resolve.length;", Value::Number(1.0));
    assert_eval("Promise.reject.length;", Value::Number(1.0));
    assert_eval(
        "Promise.resolve(1) instanceof Promise;",
        Value::Boolean(true),
    );
    assert_eval(
        "Promise.reject('x') instanceof Promise;",
        Value::Boolean(true),
    );
    assert_eval(
        "Object.prototype.toString.call(Promise.resolve(1));",
        Value::String("[object Promise]".to_owned()),
    );
    assert_eval(
        "var p = Promise.resolve(1); Promise.resolve(p) === p;",
        Value::Boolean(true),
    );
}
