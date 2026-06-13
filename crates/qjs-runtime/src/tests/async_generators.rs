//! Async generators and `for await ... of` (T007 S5, ES2023 27.6 / 14.7.5.6).
//!
//! Like the async-function tests, [`crate::eval`] drains the realm job queue
//! before returning, so values pushed into a script-returned array by promise
//! reactions are observable in the returned array (shared by reference).

use crate::{Value, eval, promise};

/// Evaluates `source` and joins the (string/number) elements of the array it
/// produced after the job queue drained.
fn eval_log(source: &str) -> String {
    match eval(source).expect("async generator evaluation should succeed") {
        Value::Array(array) => array
            .to_vec()
            .into_iter()
            .map(|value| match value {
                Value::String(text) => text,
                Value::Number(number) => number.to_string(),
                Value::Boolean(flag) => flag.to_string(),
                Value::Undefined => "undefined".to_owned(),
                other => format!("{other:?}"),
            })
            .collect::<Vec<_>>()
            .join(","),
        other => panic!("expected an array log, got {other:?}"),
    }
}

#[test]
fn calling_async_generator_runs_the_parameter_prologue_synchronously() {
    // Unlike async functions (whose parameter-binding errors reject the returned
    // promise), an async generator runs FunctionDeclarationInstantiation at the
    // call, so a throwing default initializer throws synchronously before the
    // async generator object exists. This matches QuickJS-NG.
    let result = eval(
        "async function* g([x = (() => { throw new TypeError('boom'); })()]) { yield x; } \
         g([undefined]);",
    );
    assert!(
        matches!(&result, Err(error) if error.message.contains("boom")),
        "async-generator parameter-binding error should throw at the call, got {result:?}"
    );
}

#[test]
fn calling_async_generator_returns_an_async_generator() {
    // The returned object is neither a plain promise nor a sync iterator result;
    // its next() returns a promise.
    let value = eval(
        "async function* g() { yield 1; } \
         var it = g(); typeof it.next();",
    )
    .expect("eval");
    assert_eq!(value, Value::String("object".to_owned()));
    let promise = eval(
        "async function* g() { yield 1; } \
         g().next();",
    )
    .expect("eval");
    assert!(
        promise::promise_debug_state_result(&promise).is_some(),
        "next() should return a promise, got {promise:?}"
    );
}

#[test]
fn next_chain_observes_yielded_values() {
    assert_eq!(
        eval_log(
            "var o = []; \
             async function* g() { yield 1; yield 2; } \
             var it = g(); \
             it.next().then(r => { o.push(r.value); o.push(r.done); \
               return it.next(); }).then(r => { o.push(r.value); }); \
             o;"
        ),
        "1,false,2"
    );
}

#[test]
fn for_await_of_over_async_generator() {
    assert_eq!(
        eval_log(
            "var o = []; \
             async function* g() { yield 'a'; yield 'b'; yield 'c'; } \
             async function run() { for await (const x of g()) { o.push(x); } } \
             run(); o;"
        ),
        "a,b,c"
    );
}

#[test]
fn for_await_of_over_plain_array() {
    assert_eq!(
        eval_log(
            "var o = []; \
             async function run() { for await (const x of [1, 2, 3]) { o.push(x); } } \
             run(); o;"
        ),
        "1,2,3"
    );
}

#[test]
fn await_inside_async_generator_between_yields() {
    assert_eq!(
        eval_log(
            "var o = []; \
             async function* g() { yield 1; const x = await Promise.resolve(10); yield x; } \
             async function run() { for await (const v of g()) { o.push(v); } } \
             run(); o;"
        ),
        "1,10"
    );
}

#[test]
fn yield_awaits_its_operand() {
    // A yielded promise is awaited; the consumer sees the resolved value.
    assert_eq!(
        eval_log(
            "var o = []; \
             async function* g() { yield Promise.resolve(42); } \
             async function run() { for await (const v of g()) { o.push(v); } } \
             run(); o;"
        ),
        "42"
    );
}

#[test]
fn return_mid_suspension_runs_finally() {
    assert_eq!(
        eval_log(
            "var o = []; \
             async function* g() { try { yield 1; yield 2; } finally { o.push('finally'); } } \
             var it = g(); \
             it.next().then(r => { o.push(r.value); return it.return(99); }) \
               .then(r => { o.push(r.value); o.push(r.done); }); \
             o;"
        ),
        "1,finally,99,true"
    );
}

#[test]
fn throw_into_async_generator_caught_by_body() {
    assert_eq!(
        eval_log(
            "var o = []; \
             async function* g() { try { yield 1; } catch (e) { o.push('caught:' + e); yield 2; } } \
             var it = g(); \
             it.next().then(r => { o.push(r.value); return it.throw('boom'); }) \
               .then(r => { o.push(r.value); }); \
             o;"
        ),
        "1,caught:boom,2"
    );
}

#[test]
fn overlapping_next_calls_are_fifo() {
    // Three next() calls issued before any settles must resolve in order.
    assert_eq!(
        eval_log(
            "var o = []; \
             async function* g() { yield 'x'; yield 'y'; yield 'z'; } \
             var it = g(); \
             it.next().then(r => o.push(r.value)); \
             it.next().then(r => o.push(r.value)); \
             it.next().then(r => o.push(r.value)); \
             o;"
        ),
        "x,y,z"
    );
}

#[test]
fn uncaught_body_error_rejects_pending_next() {
    assert_eq!(
        eval_log(
            "var o = []; \
             async function* g() { throw 'fail'; } \
             g().next().then(() => o.push('resolved'), e => o.push('rejected:' + e)); \
             o;"
        ),
        "rejected:fail"
    );
}

#[test]
fn completed_generator_yields_done() {
    assert_eq!(
        eval_log(
            "var o = []; \
             async function* g() { yield 1; } \
             var it = g(); \
             it.next().then(r => { o.push(r.value); return it.next(); }) \
               .then(r => { o.push(r.value); o.push(r.done); }); \
             o;"
        ),
        "1,undefined,true"
    );
}

#[test]
fn async_generator_method_in_class() {
    assert_eq!(
        eval_log(
            "var o = []; \
             class C { async *m() { yield 1; yield 2; } } \
             async function run() { for await (const v of new C().m()) { o.push(v); } } \
             run(); o;"
        ),
        "1,2"
    );
}

#[test]
fn yield_star_uses_async_iterator_protocol() {
    assert_eq!(
        eval_log(
            "var o = []; \
             var obj = { \
               get [Symbol.iterator]() { o.push('sync'); throw 'sync'; }, \
               [Symbol.asyncIterator]() { \
                 o.push('async'); \
                 return { next() { throw 'reason'; } }; \
               } \
             }; \
             async function* g() { yield* obj; } \
             g().next().then(() => o.push('fulfilled'), e => o.push(e)); \
             o;"
        ),
        "async,reason"
    );
    assert_eq!(
        eval_log(
            "var o = []; \
             var obj = { \
               get [Symbol.iterator]() { o.push('sync'); throw 'sync'; }, \
               [Symbol.asyncIterator]() { \
                 o.push('async'); \
                 return { next() { throw 'reason'; } }; \
               } \
             }; \
             class C { static async *g() { yield* obj; } } \
             C.g().next().then(() => o.push('fulfilled'), e => o.push(e)); \
             o;"
        ),
        "async,reason"
    );
}

#[test]
fn async_generator_method_in_object_literal() {
    assert_eq!(
        eval_log(
            "var o = []; \
             var obj = { async *m() { yield 7; yield 8; } }; \
             async function run() { for await (const v of obj.m()) { o.push(v); } } \
             run(); o;"
        ),
        "7,8"
    );
}

#[test]
fn symbol_async_iterator_returns_self() {
    assert_eq!(
        eval(
            "async function* g() {} \
             var it = g(); it[Symbol.asyncIterator]() === it;"
        )
        .expect("eval"),
        Value::Boolean(true)
    );
}

#[test]
fn for_await_break_closes_iterator() {
    assert_eq!(
        eval_log(
            "var o = []; \
             async function* g() { try { yield 1; yield 2; yield 3; } finally { o.push('closed'); } } \
             async function run() { \
               for await (const v of g()) { o.push(v); if (v === 2) break; } } \
             run(); o;"
        ),
        "1,2,closed"
    );
}

#[test]
fn prototype_chain_tags() {
    // The function's [[Prototype]] is %AsyncGeneratorFunction.prototype% with
    // the AsyncGeneratorFunction tag; the instance inherits %AsyncGeneratorPrototype%.
    assert_eq!(
        eval(
            "async function* g() {} \
             Object.prototype.toString.call(Object.getPrototypeOf(g));"
        )
        .expect("eval"),
        Value::String("[object AsyncGeneratorFunction]".to_owned())
    );
    assert_eq!(
        eval(
            "async function* g() {} \
             var proto = Object.getPrototypeOf(g.prototype); \
             Object.prototype.toString.call(proto);"
        )
        .expect("eval"),
        Value::String("[object AsyncGenerator]".to_owned())
    );
    assert_eq!(
        eval(
            "async function* g() {} \
             Object.getPrototypeOf(g()) === g.prototype;"
        )
        .expect("eval"),
        Value::Boolean(true)
    );
}

#[test]
fn for_await_over_sync_iterable_of_promises_awaits_values() {
    // CreateAsyncFromSyncIterator: a sync iterable whose values are promises has
    // each value awaited before the loop body sees it.
    assert_eq!(
        eval_log(
            "var o = []; \
             async function run() { \
               for await (const v of [Promise.resolve(1), Promise.resolve(2)]) { o.push(v); } } \
             run(); o;"
        ),
        "1,2"
    );
}
