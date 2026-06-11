//! Async function evaluation (T007 S3): async functions return promises that
//! settle with the body's completion, and `await` suspends and resumes via the
//! job queue. Async generators and `for await` are covered in
//! `tests/async_generators.rs` (S5).
//!
//! Runtime observation: [`crate::eval`] drains the realm job queue before
//! returning, so a `then` callback that writes into an array the script also
//! returns is observable in the returned value (the array is shared by
//! reference and mutated when reactions run during the drain).

use crate::{Value, eval, promise};

/// Evaluates `source` and returns the joined string elements of the array it
/// produced, after the job queue drained.
fn eval_log(source: &str) -> String {
    match eval(source).expect("async evaluation should succeed") {
        Value::Array(array) => array
            .to_vec()
            .into_iter()
            .map(|value| match value {
                Value::String(text) => text,
                Value::Number(number) => number.to_string(),
                other => format!("{other:?}"),
            })
            .collect::<Vec<_>>()
            .join(","),
        other => panic!("expected an array log, got {other:?}"),
    }
}

#[test]
fn async_function_returns_a_promise() {
    let value = eval("async function f() { return 1; } f();").expect("call succeeds");
    assert!(
        promise::promise_debug_state_result(&value).is_some(),
        "async call should return a promise, got {value:?}"
    );
}

#[test]
fn resolution_value_observable_after_drain() {
    assert_eq!(
        eval_log("var o = []; async function f() { return 7; } f().then(v => o.push(v)); o;"),
        "7"
    );
}

#[test]
fn await_of_non_promise_value() {
    assert_eq!(
        eval_log("var o = []; async function f() { var x = await 5; o.push(x); } f(); o;"),
        "5"
    );
}

#[test]
fn await_of_resolved_promise() {
    assert_eq!(
        eval_log(
            "var o = []; async function f() { var x = await Promise.resolve(9); o.push(x); } \
             f(); o;"
        ),
        "9"
    );
}

#[test]
fn await_of_rejected_promise_in_try_catch() {
    assert_eq!(
        eval_log(
            "var o = []; \
             async function f() { try { await Promise.reject('boom'); } catch (e) { o.push(e); } } \
             f(); o;"
        ),
        "boom"
    );
}

#[test]
fn uncaught_rejection_rejects_returned_promise() {
    assert_eq!(
        eval_log(
            "var o = []; \
             async function f() { await Promise.reject('bad'); } \
             f().then(() => o.push('ok'), e => o.push('rej:' + e)); o;"
        ),
        "rej:bad"
    );
}

#[test]
fn body_throw_before_await_rejects() {
    assert_eq!(
        eval_log(
            "var o = []; \
             async function f() { throw 'early'; } \
             f().then(() => o.push('ok'), e => o.push('rej:' + e)); o;"
        ),
        "rej:early"
    );
}

#[test]
fn multiple_sequential_awaits() {
    assert_eq!(
        eval_log(
            "var o = []; \
             async function f() { o.push(await 1); o.push(await 2); o.push(await 3); } \
             f(); o;"
        ),
        "1,2,3"
    );
}

#[test]
fn two_async_functions_interleave_fifo() {
    assert_eq!(
        eval_log(
            "var o = []; \
             async function a() { o.push('a1'); await 0; o.push('a2'); await 0; o.push('a3'); } \
             async function b() { o.push('b1'); await 0; o.push('b2'); await 0; o.push('b3'); } \
             a(); b(); o;"
        ),
        "a1,b1,a2,b2,a3,b3"
    );
}

#[test]
fn code_after_await_runs_asynchronously() {
    // The synchronous prefix runs up to the first await; everything after the
    // await is deferred to a microtask, so sync code logged afterwards runs
    // first.
    assert_eq!(
        eval_log(
            "var o = []; \
             async function f() { o.push('a'); await 0; o.push('c'); } \
             f(); o.push('b'); o;"
        ),
        "a,b,c"
    );
}

#[test]
fn async_method_with_super_after_await() {
    assert_eq!(
        eval_log(
            "var o = []; \
             class A { m() { return 'base'; } } \
             class B extends A { async m() { await 0; o.push(super.m()); } } \
             new B().m(); o;"
        ),
        "base"
    );
}

#[test]
fn async_object_method() {
    assert_eq!(
        eval_log(
            "var o = []; \
             var obj = { async m() { return await 11; } }; \
             obj.m().then(v => o.push(v)); o;"
        ),
        "11"
    );
}

#[test]
fn async_arrow_captures_this() {
    assert_eq!(
        eval_log(
            "var o = []; \
             var obj = { v: 3, run() { var f = async () => { await 0; o.push(this.v); }; \
             return f(); } }; \
             obj.run(); o;"
        ),
        "3"
    );
}

#[test]
fn parameter_binding_error_rejects() {
    // Destructuring a non-iterable parameter throws during body entry; per spec
    // the error rejects the returned promise rather than throwing synchronously.
    assert_eq!(
        eval_log(
            "var o = []; \
             async function f([x]) { o.push('body'); } \
             f(5).then(() => o.push('ok'), () => o.push('rejected')); o;"
        ),
        "rejected"
    );
}

#[test]
fn async_function_not_constructable() {
    let error = eval("async function f() {} new f();").expect_err("async is not constructable");
    assert!(
        error.message.contains("not a constructor"),
        "unexpected error: {}",
        error.message
    );
}

#[test]
fn async_function_has_no_prototype_property() {
    assert_eq!(
        eval("async function f() {} typeof f.prototype;").expect("eval"),
        Value::String("undefined".to_owned())
    );
}

#[test]
fn async_function_prototype_chain() {
    // Object.getPrototypeOf(async function) is %AsyncFunction.prototype%, whose
    // toStringTag is "AsyncFunction" and whose own prototype is
    // %Function.prototype% (so it remains callable-shaped, not constructable).
    assert_eq!(
        eval(
            "async function f() {} \
             Object.prototype.toString.call(Object.getPrototypeOf(f));"
        )
        .expect("eval"),
        Value::String("[object AsyncFunction]".to_owned())
    );
    assert_eq!(
        eval(
            "async function f() {} \
             Object.getPrototypeOf(Object.getPrototypeOf(f)) === Function.prototype;"
        )
        .expect("eval"),
        Value::Boolean(true)
    );
}

#[test]
fn async_arrow_is_not_constructable() {
    let error = eval("var f = async () => 1; new f();").expect_err("async arrow not constructable");
    assert!(
        error.message.contains("not a constructor"),
        "unexpected error: {}",
        error.message
    );
}
