use crate::{Value, eval, promise};

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

#[test]
fn evaluates_promise_then_shell() {
    assert_eval(
        "typeof Promise.prototype.then;",
        Value::String("function".to_owned()),
    );
    assert_eval("Promise.prototype.then.length;", Value::Number(2.0));
    assert_eval(
        "Promise.prototype.propertyIsEnumerable('then');",
        Value::Boolean(false),
    );
    assert_eval(
        "var p = Promise.resolve(1); var q = p.then(); q instanceof Promise && q !== p;",
        Value::Boolean(true),
    );
    assert_eval(
        "Object.prototype.toString.call(Promise.resolve(1).then());",
        Value::String("[object Promise]".to_owned()),
    );
    assert_eval(
        "var called = false; Promise.resolve(1).then(function() { called = true; }); called;",
        Value::Boolean(false),
    );
    assert!(eval("Promise.prototype.then.call({});").is_err());
    assert!(eval("Promise.prototype.then.call(3);").is_err());
}

#[test]
fn evaluates_promise_catch_shell() {
    assert_eval(
        "typeof Promise.prototype.catch;",
        Value::String("function".to_owned()),
    );
    assert_eval("Promise.prototype.catch.length;", Value::Number(1.0));
    assert_eval(
        "Promise.prototype.propertyIsEnumerable('catch');",
        Value::Boolean(false),
    );
    assert_eval(
        "var p = Promise.resolve(1); var q = p.catch(function() {}); q instanceof Promise && q !== p;",
        Value::Boolean(true),
    );
    assert_eval(
        "Object.prototype.toString.call(Promise.reject(1).catch(function() {}));",
        Value::String("[object Promise]".to_owned()),
    );
    assert!(eval("Promise.prototype.catch.call({});").is_err());
    assert!(eval("Promise.prototype.catch.call(3);").is_err());
}

#[test]
fn drains_promise_then_jobs_after_script() {
    let promise = eval("Promise.resolve(1).then(function(value) { return value + 1; });").unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&promise),
        Some(("fulfilled".to_owned(), Value::Number(2.0)))
    );

    let pending_then = eval(
        "var resolve; var p = new Promise(function(r) { resolve = r; }); var q = p.then(function(value) { return value + 1; }); resolve(3); q;",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&pending_then),
        Some(("fulfilled".to_owned(), Value::Number(4.0)))
    );
}

#[test]
fn drains_promise_catch_jobs_after_script() {
    let promise =
        eval("Promise.reject(2).catch(function(reason) { return reason + 1; });").unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&promise),
        Some(("fulfilled".to_owned(), Value::Number(3.0)))
    );
}
