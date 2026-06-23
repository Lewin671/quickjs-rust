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
                Value::String(text) => text.to_string(),
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
    assert_eq!(value, Value::String("object".to_owned().into()));
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
fn for_await_of_lexical_head_captures_fresh_binding_per_iteration() {
    assert_eq!(
        eval_log(
            "var o = []; \
             async function run() { \
               var f = []; \
               for await (let x of [1, 2, 3]) { f[x - 1] = function() { return x; }; } \
               o.push(f[0]()); o.push(f[1]()); o.push(f[2]()); \
             } \
             run(); o;"
        ),
        "1,2,3"
    );
}

#[test]
fn async_generator_write_to_outer_let_after_await_propagates() {
    // An async generator body that resumes after an `await` runs in a later
    // microtask whose caller env is not the defining frame. A write it makes to
    // an outer `let` binding past that suspension must still reach the cell the
    // outer closures read — without the generator's `CaptureWriteback` the write
    // stayed local and the observer saw the stale value. This is the root of the
    // whole `for await`/async-generator destructuring counter-update cluster.
    assert_eq!(
        eval_log(
            "var o = []; let count = 0; \
             async function* g() { await 1; count += 1; } \
             g().next().then(() => { o.push(count); }); o;"
        ),
        "1"
    );
    // The same staleness surfaced as a `for await` loop's counter update being
    // lost (the iterCount === 1 assertion across the Test262 cluster).
    assert_eq!(
        eval_log(
            "var o = []; let iterCount = 0; \
             async function* g() { for await (let x of [10, 20, 30]) { iterCount += 1; } } \
             g().next().then(() => { o.push(iterCount); }); o;"
        ),
        "3"
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
fn yield_star_awaits_async_iterator_next_result() {
    assert_eq!(
        eval_log(
            "var o = []; \
             var obj = { \
               [Symbol.asyncIterator]() { \
                 return { next() { return { get then() { throw 'reason'; } }; } }; \
               } \
             }; \
             async function* g() { yield* obj; o.push('after'); } \
             var it = g(); \
             it.next().then(() => o.push('fulfilled'), e => o.push(e)); \
             it.next().then(r => { o.push(r.value); o.push(r.done); }); \
             o;"
        ),
        "reason,undefined,true"
    );
}

#[test]
fn yield_star_sync_fallback_caches_next_method() {
    assert_eq!(
        eval_log(
            "var o = []; \
             var count = 0; \
             var obj = { \
               get [Symbol.asyncIterator]() { o.push('get async'); return null; }, \
               [Symbol.iterator]() { \
                 o.push('call sync'); \
                 return { \
                   get next() { \
                     o.push('get next'); \
                     return function() { \
                       o.push('call next'); \
                       count += 1; \
                       return { value: count, done: count > 1 }; \
                     }; \
                   } \
                 }; \
               } \
             }; \
             async function* g() { var v = yield* obj; o.push('return ' + v); } \
             var it = g(); \
             it.next().then(r => { \
               o.push(r.value); \
               return it.next('resume'); \
             }).then(r => { o.push(r.value); o.push(r.done); }); \
             o;"
        ),
        "get async,call sync,get next,call next,1,call next,return 2,undefined,true"
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
        Value::String("[object AsyncGeneratorFunction]".to_owned().into())
    );
    assert_eq!(
        eval(
            "async function* g() {} \
             var proto = Object.getPrototypeOf(g.prototype); \
             Object.prototype.toString.call(proto);"
        )
        .expect("eval"),
        Value::String("[object AsyncGenerator]".to_owned().into())
    );
    assert_eq!(
        eval(
            "async function* g() {} \
             Object.getPrototypeOf(Object.getPrototypeOf(g.prototype)) !== Object.prototype;"
        )
        .expect("eval"),
        Value::Boolean(true)
    );
    assert_eq!(
        eval(
            "async function* g() {} \
             Object.getPrototypeOf(g()) === g.prototype;"
        )
        .expect("eval"),
        Value::Boolean(true)
    );
    assert_eq!(
        eval(
            "async function* g() {} \
             g.constructor.prototype.prototype === Object.getPrototypeOf(g.prototype);"
        )
        .expect("eval"),
        Value::Boolean(true)
    );
    assert_eq!(
        eval("typeof AsyncGeneratorFunction;").expect("eval"),
        Value::String("undefined".to_owned().into())
    );
}

#[test]
fn async_generator_function_constructor_creates_async_generators() {
    assert_eq!(
        eval(
            "var AsyncGeneratorFunction = Object.getPrototypeOf(async function*() {}).constructor; \
             var fn = new AsyncGeneratorFunction('yield 1;'); \
             Object.prototype.toString.call(Object.getPrototypeOf(fn)) + ':' + \
             Object.prototype.toString.call(Object.getPrototypeOf(fn.prototype));"
        )
        .expect("eval"),
        Value::String(
            "[object AsyncGeneratorFunction]:[object AsyncGenerator]"
                .to_owned()
                .into()
        )
    );
}

#[test]
fn yield_star_async_from_sync_does_not_expose_wrapper_intrinsics() {
    assert_eq!(
        eval_log(
            "var o = []; \
             var AsyncIteratorPrototype = \
               Object.getPrototypeOf(async function*(){}.constructor.prototype.prototype); \
             Object.defineProperty(AsyncIteratorPrototype, Symbol.iterator, { \
               get: function() { throw new Error('@@iterator accessed'); } }); \
             Object.defineProperty(AsyncIteratorPrototype, Symbol.asyncIterator, { \
               get: function() { throw new Error('@@asyncIterator accessed'); } }); \
             async function* g() { yield* []; } \
             g().next().then(function() { o.push('done'); }, function(error) { o.push(error.message); }); \
             o;"
        ),
        "done"
    );
}

#[test]
fn yield_star_async_delegate_reads_not_done_value_inside_body() {
    assert_eq!(
        eval_log(
            "var o = []; \
             var iter = { \
               [Symbol.asyncIterator]: function() { return this; }, \
               next: function() { return { done: false, get value() { throw 'marker'; } }; } \
             }; \
             async function* g() { try { yield* iter; } catch (error) { return error; } } \
             g().next().then(function(result) { o.push(result.value); o.push(result.done); }); \
             o;"
        ),
        "marker,true"
    );
}

#[test]
fn yield_star_return_awaits_missing_inner_return_value() {
    assert_eq!(
        eval_log(
            "var o = []; \
             var iter = { \
               [Symbol.asyncIterator]: function() { return this; }, \
               next: function() { return { done: false, value: 1 }; } \
             }; \
             async function* g() { yield* iter; } \
             var it = g(); \
             it.next().then(function() { \
               return it.return(Promise.resolve(3)); \
             }).then(function(result) { o.push(result.value); o.push(result.done); }); \
             o;"
        ),
        "3,true"
    );
}

#[test]
fn yield_star_return_awaits_outer_return_before_inner_return_lookup() {
    assert_eq!(
        eval_log(
            "var o = []; \
             var iter = { \
               [Symbol.asyncIterator]: function() { return this; }, \
               next: function() { o.push('next'); return { done: false, value: 1 }; }, \
               get return() { o.push('get return'); return function() { return { done: true, value: 2 }; }; } \
             }; \
             var value = { get then() { o.push('get then'); } }; \
             async function* g() { yield* iter; } \
             var it = g(); \
             it.next().then(function() { o.push('returned'); it.return(value); }); \
             o;"
        ),
        "next,returned,get then,get return"
    );
}

#[test]
fn return_before_start_awaits_promise_value() {
    assert_eq!(
        eval_log(
            "var o = []; \
             var resolve; \
             var promise = new Promise(function(r) { resolve = r; }); \
             async function* g() { throw new Error('unreachable'); } \
             var it = g(); \
             it.return(promise).then(function(result) { o.push(result.value); o.push(result.done); }); \
             resolve('unwrapped'); \
             o;"
        ),
        "unwrapped,true"
    );
}

#[test]
fn completed_return_undefined_settles_after_promise_jobs() {
    assert_eq!(
        eval_log(
            "var o = []; \
             var tick = Promise.resolve().then(function() { o.push('tick 1'); }).then(function() { o.push('tick 2'); }); \
             async function* normalCompletion() {} \
             async function* bareReturn() { return; } \
             async function* explicitUndefined() { return undefined; } \
             async function* explicitVoid() { return void 0; } \
             normalCompletion().next().then(function() { o.push('normal'); }); \
             bareReturn().next().then(function() { o.push('bare'); }); \
             explicitUndefined().next().then(function() { o.push('explicit'); }); \
             explicitVoid().next().then(function() { o.push('void'); }); \
             o;"
        ),
        "tick 1,normal,bare,tick 2,explicit,void"
    );
}

#[test]
fn yield_star_return_without_inner_return_awaits_thenable_once() {
    assert_eq!(
        eval_log(
            "var o = []; \
             var asyncIter = { \
               [Symbol.asyncIterator]: function() { return this; }, \
               next: function() { return { done: false }; }, \
               get return() { o.push('get return'); } \
             }; \
             async function* g() { o.push('start'); yield* asyncIter; } \
             Promise.resolve() \
               .then(function() { o.push('tick 1'); }) \
               .then(function() { o.push('tick 2'); }) \
               .then(function() { o.push('tick 3'); }); \
             var it = g(); \
             it.next(); \
             it.return({ get then() { o.push('get then'); } }); \
             o;"
        ),
        "start,tick 1,get then,tick 2,get return,get then,tick 3"
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

#[test]
fn async_from_sync_next_omits_absent_value_argument() {
    assert_eq!(
        eval_log(
            "var o = []; \
             var iter = { \
               [Symbol.iterator]: function() { return this; }, \
               next: function() { o.push(arguments.length); return { done: true }; } \
             }; \
             async function run() { for await (const v of iter) {} } \
             run(); o;"
        ),
        "0"
    );
}

#[test]
fn async_from_sync_return_omits_absent_value_argument() {
    assert_eq!(
        eval_log(
            "var o = []; \
             var iter = { \
               [Symbol.iterator]: function() { return this; }, \
               next: function() { return { done: false, value: 1 }; }, \
               return: function() { o.push(arguments.length); return { done: true }; } \
             }; \
             async function run() { for await (const v of iter) { break; } } \
             run(); o;"
        ),
        "0"
    );
}

#[test]
fn body_thrown_native_error_rejects_with_a_real_error_object() {
    // A native error thrown by the async-generator body (here the
    // AsyncGeneratorFunction constructor rejecting an `await` in parameters)
    // must reject the `next()` promise with the materialized Error object, not
    // `undefined`. Previously the rejection reason defaulted to `undefined`
    // because the internal error carried no pre-built thrown value.
    assert_eq!(
        eval_log(
            "let log = []; \
             let AGF = Object.getPrototypeOf(async function* () {}).constructor; \
             let g = async function* () { AGF('x = await 42', ''); }; \
             g().next().then( \
               () => log.push('fulfilled'), \
               (e) => log.push(e instanceof SyntaxError) \
             ); \
             log;"
        ),
        "true"
    );
}
