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
fn evaluates_promise_all_shell() {
    assert_eval("typeof Promise.all;", Value::String("function".to_owned()));
    assert_eval("Promise.all.length;", Value::Number(1.0));
    assert_eval(
        "Promise.propertyIsEnumerable('all');",
        Value::Boolean(false),
    );
    assert_eval("Promise.all([]) instanceof Promise;", Value::Boolean(true));
    assert_eval(
        "Object.prototype.toString.call(Promise.all([]));",
        Value::String("[object Promise]".to_owned()),
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
    assert_eval(
        "var receiver = { then: function(onFulfilled, onRejected) { return typeof onFulfilled + ':' + typeof onRejected + ':' + (this === receiver); } }; Promise.prototype.catch.call(receiver, function() {});",
        Value::String("undefined:function:true".to_owned()),
    );
    assert!(eval("Promise.prototype.catch.call({});").is_err());
    assert!(eval("Promise.prototype.catch.call(3);").is_err());
}

#[test]
fn evaluates_promise_finally_shell() {
    assert_eval(
        "typeof Promise.prototype.finally;",
        Value::String("function".to_owned()),
    );
    assert_eval("Promise.prototype.finally.length;", Value::Number(1.0));
    assert_eval(
        "Promise.prototype.propertyIsEnumerable('finally');",
        Value::Boolean(false),
    );
    assert_eval(
        "var p = Promise.resolve(1); var q = p.finally(function() {}); q instanceof Promise && q !== p;",
        Value::Boolean(true),
    );
    assert_eval(
        "Object.prototype.toString.call(Promise.resolve(1).finally(function() {}));",
        Value::String("[object Promise]".to_owned()),
    );
    assert_eval(
        "var receiver = { then: function(onFulfilled, onRejected) { return typeof onFulfilled + ':' + typeof onRejected + ':' + (this === receiver); } }; Promise.prototype.finally.call(receiver, function() {});",
        Value::String("function:function:true".to_owned()),
    );
    assert_eval(
        "var receiver = { then: function(onFulfilled, onRejected) { return (onFulfilled === 1) + ':' + (onRejected === 1); } }; Promise.prototype.finally.call(receiver, 1);",
        Value::String("true:true".to_owned()),
    );
    assert!(eval("Promise.prototype.finally.call({});").is_err());
    assert!(eval("Promise.prototype.finally.call(3);").is_err());
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

#[test]
fn drains_promise_finally_jobs_after_script() {
    let fulfilled =
        eval("var calls = 0; Promise.resolve(5).finally(function() { calls = calls + 1; });")
            .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&fulfilled),
        Some(("fulfilled".to_owned(), Value::Number(5.0)))
    );

    let recovered = eval(
        "var calls = 0; Promise.reject(7).finally(function() { calls = calls + 1; }).catch(function(reason) { return reason + calls; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&recovered),
        Some(("fulfilled".to_owned(), Value::Number(8.0)))
    );

    let thrown = eval(
        "Promise.resolve(1).finally(function() { throw 9; }).catch(function(reason) { return reason; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&thrown),
        Some(("fulfilled".to_owned(), Value::Number(9.0)))
    );
}

#[test]
fn assimilates_promise_thenables_after_script() {
    let resolved = eval(
        "Promise.resolve({ then: function(resolve) { resolve(11); } }).then(function(value) { return value + 1; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&resolved),
        Some(("fulfilled".to_owned(), Value::Number(12.0)))
    );

    let constructed = eval(
        "new Promise(function(resolve) { resolve({ then: function(resolve) { resolve(13); } }); }).then(function(value) { return value + 1; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&constructed),
        Some(("fulfilled".to_owned(), Value::Number(14.0)))
    );

    let returned = eval(
        "Promise.resolve(1).then(function() { return { then: function(resolve) { resolve(15); } }; }).then(function(value) { return value + 1; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&returned),
        Some(("fulfilled".to_owned(), Value::Number(16.0)))
    );
}

#[test]
fn assimilates_promise_thenable_rejections_after_script() {
    let rejected = eval(
        "Promise.resolve({ then: function(resolve, reject) { reject(21); } }).catch(function(reason) { return reason + 1; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&rejected),
        Some(("fulfilled".to_owned(), Value::Number(22.0)))
    );

    let first_settlement = eval(
        "Promise.resolve({ then: function(resolve, reject) { resolve(31); reject(32); } }).then(function(value) { return value; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&first_settlement),
        Some(("fulfilled".to_owned(), Value::Number(31.0)))
    );

    let poisoned = eval(
        "var value = {}; Object.defineProperty(value, 'then', { get: function() { throw 41; } }); Promise.resolve(value).catch(function(reason) { return reason + 1; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&poisoned),
        Some(("fulfilled".to_owned(), Value::Number(42.0)))
    );
}

#[test]
fn fulfills_non_callable_then_values_after_script() {
    let fulfilled = eval(
        "var value = { then: 1, marker: 51 }; Promise.resolve(value).then(function(result) { return result.marker + 1; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&fulfilled),
        Some(("fulfilled".to_owned(), Value::Number(52.0)))
    );
}

#[test]
fn drains_promise_all_jobs_after_script() {
    let empty = eval("Promise.all([]);").unwrap();
    let Some((state, Value::Array(values))) = promise::promise_debug_state_result(&empty) else {
        panic!("Promise.all([]) should fulfill with an array");
    };
    assert_eq!(state, "fulfilled");
    assert_eq!(values.len(), 0);

    let mixed = eval(
        "Promise.all([Promise.resolve(1), 2, { then: function(resolve) { resolve(3); } }]).then(function(values) { return values.join(':'); });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&mixed),
        Some(("fulfilled".to_owned(), Value::String("1:2:3".to_owned())))
    );

    let first_settlement = eval(
        "Promise.all([{ then: function(resolve, reject) { resolve(4); reject(5); } }]).then(function(values) { return values[0]; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&first_settlement),
        Some(("fulfilled".to_owned(), Value::Number(4.0)))
    );
}

#[test]
fn drains_promise_all_rejections_after_script() {
    let rejected = eval(
        "Promise.all([Promise.resolve(1), Promise.reject(2)]).catch(function(reason) { return reason + 1; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&rejected),
        Some(("fulfilled".to_owned(), Value::Number(3.0)))
    );
}
