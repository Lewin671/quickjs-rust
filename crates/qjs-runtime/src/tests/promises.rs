use crate::{Value, eval, eval_keep_jobs, promise};

fn assert_eval(source: &str, expected: Value) {
    assert_eq!(eval(source), Ok(expected));
}

/// Asserts the resolved value of the script's final promise, which records
/// microtask execution order into the accumulator it eventually resolves with.
fn assert_job_order(source: &str, expected: &str) {
    let promise = eval(source).unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&promise),
        Some((
            "fulfilled".to_owned(),
            Value::String(expected.to_owned().into())
        ))
    );
}

#[test]
fn evaluates_promise_constructor_shell() {
    assert_eval(
        "typeof Promise;",
        Value::String("function".to_owned().into()),
    );
    assert_eval("Promise.length;", Value::Number(1.0));
    assert_eval(
        "new Promise(function(resolve) { resolve(1); }) instanceof Promise;",
        Value::Boolean(true),
    );
    assert_eval(
        "Object.prototype.toString.call(new Promise(function(resolve) { resolve(1); }));",
        Value::String("[object Promise]".to_owned().into()),
    );
    assert_eval(
        "var called = false; new Promise(function(resolve, reject) { called = typeof resolve + ':' + typeof reject; resolve(1); }); called;",
        Value::String("function:function".to_owned().into()),
    );
    assert!(eval("Promise(function() {});").is_err());
    assert!(eval("new Promise(1);").is_err());
}

#[test]
fn promise_constructor_propagates_new_target_prototype_getter_error() {
    assert_eval(
        "var bound = (function() {}).bind();\
         var seen = false;\
         Object.defineProperty(bound, 'prototype', { get: function() { seen = true; throw new Error('prototype'); } });\
         try { Reflect.construct(Promise, [function() {}], bound); 'no throw'; } catch (error) { seen + ':' + error.message; }",
        Value::String("true:prototype".to_owned().into()),
    );
    assert_eval(
        "var bound = (function() {}).bind();\
         var seen = false;\
         Object.defineProperty(bound, 'prototype', { get: function() { seen = true; throw new Error('prototype'); } });\
         try { Reflect.construct(Promise, [], bound); 'no throw'; } catch (error) { seen + ':' + (error instanceof TypeError); }",
        Value::String("false:true".to_owned().into()),
    );
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
        Value::String("[object Promise]".to_owned().into()),
    );
    assert_eval(
        "var p = Promise.resolve(1); Promise.resolve(p) === p;",
        Value::Boolean(true),
    );
}

#[test]
fn promise_resolve_reject_require_object_receiver() {
    assert!(eval("Promise.resolve.call(undefined, 1);").is_err());
    assert!(eval("Promise.resolve.call(86, 1);").is_err());
    assert!(eval("Promise.reject.call(null, 1);").is_err());
}

#[test]
fn promise_resolve_returns_same_promise_only_for_matching_constructor() {
    assert_eval(
        "var p = Promise.resolve(1); Promise.resolve(p) === p;",
        Value::Boolean(true),
    );
    assert_eval(
        "var p = new Promise(function(){}); p.constructor = null; Promise.resolve(p) === p;",
        Value::Boolean(false),
    );
}

#[test]
fn promise_static_methods_use_this_constructor() {
    // Promise.resolve.call(C) constructs through C with a capabilities executor.
    assert_eval(
        "var calls = 0; function C(exec){ calls++; exec(function(){}, function(){}); }\
         C.prototype = Promise.prototype;\
         Promise.resolve.call(C, 1); calls;",
        Value::Number(1.0),
    );
    // A capabilities executor that supplies a non-callable resolve/reject throws.
    assert!(
        eval("Promise.resolve.call(function(exec){ exec(undefined, undefined); }, 1);").is_err()
    );
}

#[test]
fn promise_has_symbol_species_accessor() {
    assert_eval("Promise[Symbol.species] === Promise;", Value::Boolean(true));
    assert_eval(
        "Object.getOwnPropertyDescriptor(Promise, Symbol.species).set;",
        Value::Undefined,
    );
    assert_eval(
        "Object.getOwnPropertyDescriptor(Promise, Symbol.species).get.name;",
        Value::String("get [Symbol.species]".to_owned().into()),
    );
}

#[test]
fn promise_then_uses_species_constructor() {
    // then() builds its result promise via SpeciesConstructor of the receiver.
    assert_eval(
        "var observed; class P extends Promise {}\
         Object.defineProperty(P, Symbol.species, { value: function(exec){ observed = true; exec(function(){}, function(){}); } });\
         var p = P.resolve(1); p.then(); observed === true;",
        Value::Boolean(true),
    );
}

#[test]
fn evaluates_promise_all_shell() {
    assert_eval(
        "typeof Promise.all;",
        Value::String("function".to_owned().into()),
    );
    assert_eval("Promise.all.length;", Value::Number(1.0));
    assert_eval(
        "Promise.propertyIsEnumerable('all');",
        Value::Boolean(false),
    );
    assert_eval("Promise.all([]) instanceof Promise;", Value::Boolean(true));
    assert_eval(
        "Object.prototype.toString.call(Promise.all([]));",
        Value::String("[object Promise]".to_owned().into()),
    );
}

#[test]
fn promise_all_resolves_with_value_array() {
    assert_job_order(
        "Promise.all([1, Promise.resolve(2), 3]).then(function(v){ return v.join(','); });",
        "1,2,3",
    );
}

#[test]
fn promise_all_rejects_on_first_rejection() {
    assert_job_order(
        "Promise.all([1, Promise.reject('boom')]).then(null, function(r){ return r; });",
        "boom",
    );
}

#[test]
fn promise_all_reads_constructor_resolve_once_per_element() {
    // promiseResolve = Get(C, "resolve") is read once and called per element.
    assert_eval(
        "var getCount = 0, callCount = 0; var real = Promise.resolve;\
         Object.defineProperty(Promise, 'resolve', { configurable: true, get: function(){ getCount++; return function(){ callCount++; return real.apply(Promise, arguments); }; } });\
         Promise.all([1, 2, 3]); getCount + ':' + callCount;",
        Value::String("1:3".to_owned().into()),
    );
}

#[test]
fn promise_all_rejects_with_type_error_for_non_iterable() {
    assert_job_order(
        "Promise.all(null).then(null, function(r){ return r instanceof TypeError ? 'type-error' : 'other'; });",
        "type-error",
    );
}

#[test]
fn promise_all_settled_result_objects_have_spec_shape() {
    assert_job_order(
        "Promise.allSettled([Promise.resolve(1), Promise.reject(2)]).then(function(v){\
           return Object.keys(v[0]).join(',') + '|' + Object.keys(v[1]).join(',') + '|' + v[0].status + ',' + v[0].value + ',' + v[1].status + ',' + v[1].reason;\
         });",
        "status,value|status,reason|fulfilled,1,rejected,2",
    );
}

#[test]
fn promise_all_settled_result_objects_inherit_object_prototype() {
    assert_job_order(
        "Promise.allSettled([Promise.resolve(1)]).then(function(v){ return Object.getPrototypeOf(v[0]) === Object.prototype ? 'ok' : 'no'; });",
        "ok",
    );
}

#[test]
fn promise_any_fulfils_with_first_value() {
    assert_job_order(
        "Promise.any([Promise.reject(1), Promise.resolve('ok')]).then(function(v){ return v; });",
        "ok",
    );
}

#[test]
fn promise_any_rejects_with_aggregate_error() {
    assert_job_order(
        "Promise.any([Promise.reject(1), Promise.reject(2)]).then(null, function(e){ return (e instanceof AggregateError) + ':' + e.errors.join(','); });",
        "true:1,2",
    );
}

#[test]
fn promise_race_settles_with_first_settlement() {
    assert_job_order(
        "Promise.race([Promise.resolve('a'), Promise.resolve('b')]).then(function(v){ return v; });",
        "a",
    );
}

#[test]
fn promise_combinators_use_this_constructor() {
    // Promise.all.call(C, ...) constructs the result through C.
    assert_eval(
        "var built = false; function C(exec){ built = true; exec(function(){}, function(){}); }\
         C.resolve = function(v){ return Promise.resolve(v); };\
         Promise.all.call(C, []); built;",
        Value::Boolean(true),
    );
}

#[test]
fn promise_self_resolution_rejects_with_type_error() {
    assert_job_order(
        "var q = Promise.resolve().then(function(){ return q; });\
         q.then(null, function(e){ return e instanceof TypeError ? 'type-error' : 'other'; });",
        "type-error",
    );
}

#[test]
fn promise_constructor_resolving_functions_are_anonymous() {
    assert_eval(
        "var r; new Promise(function(resolve){ r = resolve; }); r.name + ':' + r.length;",
        Value::String(":1".to_owned().into()),
    );
}

#[test]
fn promise_resolve_then_throw_in_executor_is_ignored() {
    // resolve(value) sets alreadyResolved, so the later throw cannot reject.
    assert_job_order(
        "new Promise(function(resolve){ resolve('done'); throw new Error('ignored'); }).then(function(v){ return v; });",
        "done",
    );
}

#[test]
fn promise_finally_awaits_returned_thenable_before_forwarding() {
    // finally's onFinally returning a rejected promise overrides the fulfilment.
    assert_job_order(
        "Promise.resolve('value').finally(function(){ return Promise.reject('boom'); })\
           .then(function(){ return 'resolved'; }, function(r){ return 'rejected:' + r; });",
        "rejected:boom",
    );
}

#[test]
fn promise_finally_forwards_original_value() {
    assert_job_order(
        "Promise.resolve('value').finally(function(){ return 99; }).then(function(v){ return v; });",
        "value",
    );
}

#[test]
fn promise_finally_invokes_then_on_proxy_receiver() {
    // finally accepts any object receiver, forwarding to its `then`.
    assert_eval(
        "var called = false; var p = new Proxy(Promise.resolve(), {});\
         Promise.prototype.then = function(){ called = true; };\
         Promise.prototype.finally.call(p, function(){}); called;",
        Value::Boolean(true),
    );
}

#[test]
fn promise_with_resolvers_uses_this_constructor() {
    assert_eval(
        "class P extends Promise {} var r = Promise.withResolvers.call(P); r.promise instanceof P;",
        Value::Boolean(true),
    );
}

#[test]
fn promise_try_resolves_and_uses_this_constructor() {
    assert_job_order(
        "Promise.try(function(){ return 'ok'; }).then(function(v){ return v; });",
        "ok",
    );
    assert_eval(
        "class P extends Promise {} Promise.try.call(P, function(){}) instanceof P;",
        Value::Boolean(true),
    );
}

#[test]
fn evaluates_promise_any_shell() {
    assert_eval(
        "typeof Promise.any;",
        Value::String("function".to_owned().into()),
    );
    assert_eval("Promise.any.length;", Value::Number(1.0));
    assert_eval(
        "Promise.propertyIsEnumerable('any');",
        Value::Boolean(false),
    );
    assert_eval("Promise.any([]) instanceof Promise;", Value::Boolean(true));
    assert_eval(
        "Object.prototype.toString.call(Promise.any([]));",
        Value::String("[object Promise]".to_owned().into()),
    );
}

#[test]
fn evaluates_promise_try_shell() {
    assert_eval(
        "typeof Promise.try;",
        Value::String("function".to_owned().into()),
    );
    assert_eval("Promise.try.length;", Value::Number(1.0));
    assert_eval(
        "Promise.propertyIsEnumerable('try');",
        Value::Boolean(false),
    );
    assert_eval(
        "Promise.try(function() {}) instanceof Promise;",
        Value::Boolean(true),
    );
}

#[test]
fn evaluates_promise_with_resolvers_shell() {
    assert_eval(
        "typeof Promise.withResolvers;",
        Value::String("function".to_owned().into()),
    );
    assert_eval("Promise.withResolvers.length;", Value::Number(0.0));
    assert_eval(
        "Promise.propertyIsEnumerable('withResolvers');",
        Value::Boolean(false),
    );
    assert_eval(
        "var c = Promise.withResolvers(); c.promise instanceof Promise;",
        Value::Boolean(true),
    );
    assert_eval(
        "var c = Promise.withResolvers(); typeof c.resolve + ':' + c.resolve.length + ':' + c.resolve.name + ':' + typeof c.reject + ':' + c.reject.length + ':' + c.reject.name;",
        Value::String("function:1::function:1:".to_owned().into()),
    );
}

#[test]
fn evaluates_promise_all_settled_shell() {
    assert_eval(
        "typeof Promise.allSettled;",
        Value::String("function".to_owned().into()),
    );
    assert_eval("Promise.allSettled.length;", Value::Number(1.0));
    assert_eval(
        "Promise.propertyIsEnumerable('allSettled');",
        Value::Boolean(false),
    );
    assert_eval(
        "Promise.allSettled([]) instanceof Promise;",
        Value::Boolean(true),
    );
    assert_eval(
        "Object.prototype.toString.call(Promise.allSettled([]));",
        Value::String("[object Promise]".to_owned().into()),
    );
}

#[test]
fn evaluates_promise_race_shell() {
    assert_eval(
        "typeof Promise.race;",
        Value::String("function".to_owned().into()),
    );
    assert_eval("Promise.race.length;", Value::Number(1.0));
    assert_eval(
        "Promise.propertyIsEnumerable('race');",
        Value::Boolean(false),
    );
    assert_eval("Promise.race([]) instanceof Promise;", Value::Boolean(true));
    assert_eval(
        "Object.prototype.toString.call(Promise.race([]));",
        Value::String("[object Promise]".to_owned().into()),
    );
}

#[test]
fn evaluates_promise_then_shell() {
    assert_eval(
        "typeof Promise.prototype.then;",
        Value::String("function".to_owned().into()),
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
        Value::String("[object Promise]".to_owned().into()),
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
        Value::String("function".to_owned().into()),
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
        Value::String("[object Promise]".to_owned().into()),
    );
    assert_eval(
        "var receiver = { then: function(onFulfilled, onRejected) { return typeof onFulfilled + ':' + typeof onRejected + ':' + (this === receiver); } }; Promise.prototype.catch.call(receiver, function() {});",
        Value::String("undefined:function:true".to_owned().into()),
    );
    assert!(eval("Promise.prototype.catch.call({});").is_err());
    assert!(eval("Promise.prototype.catch.call(3);").is_err());
}

#[test]
fn evaluates_promise_finally_shell() {
    assert_eval(
        "typeof Promise.prototype.finally;",
        Value::String("function".to_owned().into()),
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
        Value::String("[object Promise]".to_owned().into()),
    );
    assert_eval(
        "var receiver = { then: function(onFulfilled, onRejected) { return typeof onFulfilled + ':' + typeof onRejected + ':' + (this === receiver); } }; Promise.prototype.finally.call(receiver, function() {});",
        Value::String("function:function:true".to_owned().into()),
    );
    assert_eval(
        "var receiver = { then: function(onFulfilled, onRejected) { return (onFulfilled === 1) + ':' + (onRejected === 1); } }; Promise.prototype.finally.call(receiver, 1);",
        Value::String("true:true".to_owned().into()),
    );
    assert!(eval("Promise.prototype.finally.call({});").is_err());
    assert!(eval("Promise.prototype.finally.call(3);").is_err());
}

#[test]
fn then_on_settled_promise_runs_asynchronously() {
    // The reaction for an already-fulfilled promise must be queued, so the
    // synchronous push runs first and the recorded order is "sync,then".
    assert_job_order(
        "var order = []; Promise.resolve(1).then(function() { order.push('then'); }); order.push('sync'); Promise.resolve().then(function() { return order.join(','); });",
        "sync,then",
    );
}

#[test]
fn reactions_run_fifo_across_promises() {
    // Two independent settled promises enqueue in source order; their
    // reactions run FIFO, ahead of the later trailing reaction that reports.
    assert_job_order(
        "var order = []; Promise.resolve('a').then(function() { order.push('a'); }); Promise.resolve('b').then(function() { order.push('b'); }); Promise.resolve().then(function() {}).then(function() { return order.join(','); });",
        "a,b",
    );
}

#[test]
fn chained_then_reactions_run_in_order() {
    // A `.then` chain enqueues each link only after the previous link runs, so
    // the chain interleaves one tick at a time with an independent chain.
    assert_job_order(
        "var order = []; Promise.resolve().then(function() { order.push('x1'); }).then(function() { order.push('x2'); }); Promise.resolve().then(function() { order.push('y1'); }).then(function() { order.push('y2'); }); Promise.resolve().then(function(){}).then(function(){}).then(function() { return order.join(','); });",
        "x1,y1,x2,y2",
    );
}

#[test]
fn thenable_resolution_jobs_run_after_queued_reactions() {
    // Assimilating a thenable enqueues a job to call its `then`; that job runs
    // after reactions already queued ahead of it, then its resolve schedules
    // the dependent reaction one tick later still.
    assert_job_order(
        "var order = []; Promise.resolve('plain').then(function() { order.push('plain'); }); var thenable = { then: function(resolve) { order.push('thenableThen'); resolve('t'); } }; Promise.resolve(thenable).then(function() { order.push('thenT'); }); Promise.resolve().then(function(){}).then(function(){}).then(function(){}).then(function() { return order.join(','); });",
        "plain,thenableThen,thenT",
    );
}

#[test]
fn jobs_are_drained_between_evaluations() {
    // Each `eval` drives its own realm and drains its queue before returning;
    // no jobs leak into a later evaluation.
    let first = eval(
        "var order = []; Promise.resolve().then(function() { order.push('first'); }); Promise.resolve().then(function(){}).then(function() { return order.join(','); });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&first),
        Some((
            "fulfilled".to_owned(),
            Value::String("first".to_owned().into())
        ))
    );
    let second = eval(
        "var order = []; Promise.resolve().then(function() { order.push('second'); }); Promise.resolve().then(function(){}).then(function() { return order.join(','); });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&second),
        Some((
            "fulfilled".to_owned(),
            Value::String("second".to_owned().into())
        ))
    );
}

#[test]
fn keep_jobs_defers_reactions_until_run_jobs() {
    // The explicit-drain API leaves reactions pending until `run_jobs` is
    // invoked, which the async test harness needs to control drain timing.
    let mut outcome = eval_keep_jobs(
        "var order = []; globalThis.order = order; Promise.resolve().then(function() { order.push('deferred'); }); order.join(',');",
    )
    .unwrap();
    assert_eq!(
        outcome.value,
        Value::String(::std::rc::Rc::new(String::new()))
    );
    outcome.run_jobs().unwrap();
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

    let proxied = eval(
        "let calls = []; \
         let thenable = new Proxy({ then: function(resolve) { calls.push('call'); resolve(17); } }, { \
           get: function(target, key, receiver) { calls.push(String(key)); return Reflect.get(target, key, receiver); } \
         }); \
         Promise.resolve(thenable).then(function(value) { return calls.join(',') + ':' + value; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&proxied),
        Some((
            "fulfilled".to_owned(),
            Value::String("then,call:17".to_owned().into())
        ))
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
        Some((
            "fulfilled".to_owned(),
            Value::String("1:2:3".to_owned().into())
        ))
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

#[test]
fn drains_promise_any_jobs_after_script() {
    let resolved = eval(
        "Promise.any([Promise.reject(1), Promise.resolve(2)]).then(function(value) { return value + 1; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&resolved),
        Some(("fulfilled".to_owned(), Value::Number(3.0)))
    );

    let non_promise =
        eval("Promise.any([1, Promise.reject(2)]).then(function(value) { return value; });")
            .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&non_promise),
        Some(("fulfilled".to_owned(), Value::Number(1.0)))
    );

    let thenable = eval(
        "Promise.any([{ then: function(resolve, reject) { resolve(4); reject(5); } }]).then(function(value) { return value; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&thenable),
        Some(("fulfilled".to_owned(), Value::Number(4.0)))
    );
}

#[test]
fn drains_promise_any_rejections_after_script() {
    let empty = eval(
        "Promise.any([]).catch(function(error) { return (error instanceof AggregateError) + ':' + error.errors.length; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&empty),
        Some((
            "fulfilled".to_owned(),
            Value::String("true:0".to_owned().into())
        ))
    );

    let rejected = eval(
        "Promise.any([Promise.reject('a'), Promise.reject('b')]).catch(function(error) { return (error instanceof AggregateError) + ':' + error.errors.join(''); });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&rejected),
        Some((
            "fulfilled".to_owned(),
            Value::String("true:ab".to_owned().into())
        ))
    );

    let poisoned = eval(
        "var value = {}; Object.defineProperty(value, 'then', { get: function() { throw 8; } }); Promise.any([value]).catch(function(error) { return (error instanceof AggregateError) + ':' + error.errors[0]; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&poisoned),
        Some((
            "fulfilled".to_owned(),
            Value::String("true:8".to_owned().into())
        ))
    );
}

#[test]
fn drains_promise_try_after_script() {
    let fulfilled = eval("Promise.try(function(a, b) { return a + b; }, 2, 3);").unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&fulfilled),
        Some(("fulfilled".to_owned(), Value::Number(5.0)))
    );

    let args = eval(
        "Promise.try(function(a, b, c) { return String(a) + ':' + b + ':' + (c === undefined); }, 1, 2);",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&args),
        Some((
            "fulfilled".to_owned(),
            Value::String("1:2:true".to_owned().into())
        ))
    );

    let rejected = eval("Promise.try(function() { throw 7; });").unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&rejected),
        Some(("rejected".to_owned(), Value::Number(7.0)))
    );

    let thenable =
        eval("Promise.try(function() { return { then: function(resolve) { resolve(11); } }; });")
            .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&thenable),
        Some(("fulfilled".to_owned(), Value::Number(11.0)))
    );
}

#[test]
fn drains_promise_with_resolvers_after_script() {
    let resolved = eval(
        "var c = Promise.withResolvers(); var p = c.promise.then(function(value) { return value + 1; }); c.resolve(4); p;",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&resolved),
        Some(("fulfilled".to_owned(), Value::Number(5.0)))
    );

    let rejected = eval(
        "var c = Promise.withResolvers(); var p = c.promise.catch(function(reason) { return reason + 1; }); c.reject(6); p;",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&rejected),
        Some(("fulfilled".to_owned(), Value::Number(7.0)))
    );

    let first_settlement = eval(
        "var c = Promise.withResolvers(); var p = c.promise.then(function(value) { return value; }); c.resolve(8); c.reject(9); p;",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&first_settlement),
        Some(("fulfilled".to_owned(), Value::Number(8.0)))
    );
}

#[test]
fn drains_promise_all_settled_jobs_after_script() {
    let empty = eval("Promise.allSettled([]);").unwrap();
    let Some((state, Value::Array(values))) = promise::promise_debug_state_result(&empty) else {
        panic!("Promise.allSettled([]) should fulfill with an array");
    };
    assert_eq!(state, "fulfilled");
    assert_eq!(values.len(), 0);

    let fulfilled = eval(
        "Promise.allSettled([Promise.resolve(1), 2]).then(function(values) { return values[0].status + ':' + values[0].value + ':' + values[1].status + ':' + values[1].value; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&fulfilled),
        Some((
            "fulfilled".to_owned(),
            Value::String("fulfilled:1:fulfilled:2".to_owned().into())
        ))
    );

    let mixed = eval(
        "Promise.allSettled([Promise.reject(3), Promise.resolve(4)]).then(function(values) { return values[0].status + ':' + values[0].reason + ':' + values[1].status + ':' + values[1].value; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&mixed),
        Some((
            "fulfilled".to_owned(),
            Value::String("rejected:3:fulfilled:4".to_owned().into())
        ))
    );

    let first_settlement = eval(
        "Promise.allSettled([{ then: function(resolve, reject) { resolve(5); reject(6); } }]).then(function(values) { return values[0].status + ':' + values[0].value; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&first_settlement),
        Some((
            "fulfilled".to_owned(),
            Value::String("fulfilled:5".to_owned().into())
        ))
    );
}

#[test]
fn drains_promise_all_settled_thenable_rejections_after_script() {
    let rejected = eval(
        "Promise.allSettled([{ then: function(resolve, reject) { reject(7); } }]).then(function(values) { return values[0].status + ':' + values[0].reason; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&rejected),
        Some((
            "fulfilled".to_owned(),
            Value::String("rejected:7".to_owned().into())
        ))
    );

    let poisoned = eval(
        "var value = {}; Object.defineProperty(value, 'then', { get: function() { throw 8; } }); Promise.allSettled([value]).then(function(values) { return values[0].status + ':' + values[0].reason; });",
    )
    .unwrap();
    assert_eq!(
        promise::promise_debug_state_result(&poisoned),
        Some((
            "fulfilled".to_owned(),
            Value::String("rejected:8".to_owned().into())
        ))
    );
}
