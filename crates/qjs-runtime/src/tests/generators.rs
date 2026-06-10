use crate::{Value, eval};

fn number(source: &str) -> f64 {
    match eval(source) {
        Ok(Value::Number(value)) => value,
        other => panic!("expected number from {source:?}, got {other:?}"),
    }
}

fn boolean(source: &str) -> bool {
    match eval(source) {
        Ok(Value::Boolean(value)) => value,
        other => panic!("expected boolean from {source:?}, got {other:?}"),
    }
}

fn string(source: &str) -> String {
    match eval(source) {
        Ok(Value::String(value)) => value,
        other => panic!("expected string from {source:?}, got {other:?}"),
    }
}

#[test]
fn calling_a_generator_does_not_run_the_body() {
    // The body's side effect must not run until the first `next`.
    assert_eq!(
        number("let ran = 0; function* g() { ran = 1; yield 1; } let it = g(); ran;"),
        0.0
    );
    assert_eq!(
        number("let ran = 0; function* g() { ran = 1; yield 1; } let it = g(); it.next(); ran;"),
        1.0
    );
}

#[test]
fn yields_values_in_sequence() {
    assert_eq!(
        number(
            "function* g() { yield 1; yield 2; yield 3; } \
             let it = g(); it.next().value + it.next().value * 10 + it.next().value * 100;"
        ),
        321.0
    );
    assert!(boolean(
        "function* g() { yield 1; } let it = g(); it.next(); it.next().done;"
    ));
    assert!(!boolean(
        "function* g() { yield 1; yield 2; } let it = g(); it.next().done;"
    ));
}

#[test]
fn first_next_argument_is_ignored() {
    // The argument to the first `next` cannot be observed by the body.
    assert_eq!(
        number(
            "function* g() { let x = yield 1; yield x; } \
             let it = g(); it.next(99); it.next(7).value;"
        ),
        7.0
    );
}

#[test]
fn resume_value_becomes_yield_result() {
    assert_eq!(
        number(
            "function* g() { let a = yield 1; let b = yield a + 1; return a + b; } \
             let it = g(); it.next(); let r = it.next(10); it.next(20); r.value;"
        ),
        11.0
    );
}

#[test]
fn return_completion_at_end() {
    assert!(boolean(
        "function* g() { yield 1; return 42; } \
         let it = g(); it.next(); it.next().done;"
    ));
    assert_eq!(
        number(
            "function* g() { yield 1; return 42; } \
             let it = g(); it.next(); it.next().value;"
        ),
        42.0
    );
    // A return value is delivered once; the following next is undefined/done.
    assert!(boolean(
        "function* g() { return 5; } let it = g(); it.next(); it.next().value === undefined;"
    ));
}

#[test]
fn generator_with_no_yields() {
    assert!(boolean("function* g() { } let it = g(); it.next().done;"));
    assert!(boolean(
        "function* g() { } let it = g(); it.next().value === undefined;"
    ));
}

#[test]
fn independent_instances() {
    assert_eq!(
        number(
            "function* g() { yield 1; yield 2; } \
             let a = g(); let b = g(); a.next(); a.next().value * 10 + b.next().value;"
        ),
        21.0
    );
}

#[test]
fn for_of_over_a_generator() {
    assert_eq!(
        number(
            "function* g() { yield 1; yield 2; yield 3; } \
             let sum = 0; for (const x of g()) { sum += x; } sum;"
        ),
        6.0
    );
}

#[test]
fn for_of_early_break_calls_return() {
    // Breaking out of a for-of closes the iterator, running the generator's
    // finally block.
    assert_eq!(
        number(
            "let closed = 0; \
             function* g() { try { yield 1; yield 2; yield 3; } finally { closed = 1; } } \
             for (const x of g()) { if (x === 2) break; } closed;"
        ),
        1.0
    );
}

#[test]
fn symbol_iterator_returns_self() {
    assert!(boolean(
        "function* g() { yield 1; } let it = g(); it[Symbol.iterator]() === it;"
    ));
}

#[test]
fn to_string_tag_is_generator() {
    assert_eq!(
        string("function* g() {} Object.prototype.toString.call(g());"),
        "[object Generator]"
    );
}

#[test]
fn return_before_start() {
    assert!(boolean(
        "function* g() { yield 1; } let it = g(); it.return(7).done;"
    ));
    assert_eq!(
        number("function* g() { yield 1; } let it = g(); it.return(7).value;"),
        7.0
    );
    // After an early return the body never runs.
    assert_eq!(
        number(
            "let ran = 0; function* g() { ran = 1; yield 1; } \
             let it = g(); it.return(7); ran;"
        ),
        0.0
    );
}

#[test]
fn return_after_completion() {
    assert!(boolean(
        "function* g() { yield 1; } \
         let it = g(); it.next(); it.next(); it.return(9).done;"
    ));
    assert_eq!(
        number(
            "function* g() { yield 1; } \
             let it = g(); it.next(); it.next(); it.return(9).value;"
        ),
        9.0
    );
}

#[test]
fn return_mid_yield_without_finally() {
    assert!(boolean(
        "function* g() { yield 1; yield 2; } \
         let it = g(); it.next(); it.return(3).done;"
    ));
    assert_eq!(
        number(
            "function* g() { yield 1; yield 2; } \
             let it = g(); it.next(); it.return(3).value;"
        ),
        3.0
    );
    // The generator is completed after a mid-yield return.
    assert!(boolean(
        "function* g() { yield 1; yield 2; } \
         let it = g(); it.next(); it.return(3); it.next().done;"
    ));
}

#[test]
fn return_mid_yield_runs_finally() {
    assert_eq!(
        number(
            "let cleaned = 0; \
             function* g() { try { yield 1; yield 2; } finally { cleaned = 5; } } \
             let it = g(); it.next(); it.return(3); cleaned;"
        ),
        5.0
    );
}

#[test]
fn finally_can_override_return_completion() {
    assert_eq!(
        number(
            "function* g() { try { yield 1; } finally { return 99; } } \
             let it = g(); it.next(); it.return(3).value;"
        ),
        99.0
    );
}

#[test]
fn throw_before_start() {
    assert_eq!(
        number(
            "function* g() { yield 1; } let it = g(); \
             try { it.throw(42); } catch (e) { e } "
        ),
        42.0
    );
    // The body never ran.
    assert_eq!(
        number(
            "let ran = 0; function* g() { ran = 1; yield 1; } let it = g(); \
             try { it.throw(1); } catch (e) {} ran;"
        ),
        0.0
    );
}

#[test]
fn throw_mid_yield_caught_by_body() {
    assert_eq!(
        number(
            "function* g() { \
                try { yield 1; } catch (e) { yield e + 1; } \
             } \
             let it = g(); it.next(); it.throw(10).value;"
        ),
        11.0
    );
    // After catching, the generator can continue normally.
    assert!(boolean(
        "function* g() { try { yield 1; } catch (e) {} } \
         let it = g(); it.next(); it.throw(1); it.next().done;"
    ));
}

#[test]
fn throw_uncaught_propagates_and_completes() {
    assert_eq!(
        number(
            "function* g() { yield 1; yield 2; } \
             let it = g(); it.next(); \
             try { it.throw(7); } catch (e) { e }"
        ),
        7.0
    );
    // An uncaught throw completes the generator.
    assert!(boolean(
        "function* g() { yield 1; yield 2; } \
         let it = g(); it.next(); \
         try { it.throw(7); } catch (e) {} it.next().done;"
    ));
}

#[test]
fn reentrant_next_is_type_error() {
    // Calling `next` on a generator while its body is running is a TypeError.
    // The body reaches itself through a captured holder object so the guard is
    // exercised (not an undefined-identifier reference).
    assert!(boolean(
        "let holder = {}; \
         function* g() { holder.it.next(); yield 1; } \
         holder.it = g(); \
         let caught = false; \
         try { holder.it.next(); } catch (e) { caught = e instanceof TypeError; } caught;"
    ));
}

#[test]
fn new_on_generator_is_type_error() {
    assert!(boolean(
        "function* g() { yield 1; } \
         let caught = false; \
         try { new g(); } catch (e) { caught = e instanceof TypeError; } caught;"
    ));
}

#[test]
fn yield_in_argument_and_member_position() {
    // `yield` in argument position resumes with the passed value.
    assert_eq!(
        number(
            "function id(x) { return x; } \
             function* g() { return id(yield 1); } \
             let it = g(); it.next(); it.next(8).value;"
        ),
        8.0
    );
    // `yield` as a computed member key.
    assert_eq!(
        number(
            "function* g() { let o = { a: 5 }; return o[yield 1]; } \
             let it = g(); it.next(); it.next('a').value;"
        ),
        5.0
    );
}

#[test]
fn object_literal_generator_method() {
    assert_eq!(
        number(
            "let o = { *gen() { yield 1; yield 2; } }; \
             let it = o.gen(); it.next().value * 10 + it.next().value;"
        ),
        12.0
    );
}

#[test]
fn class_generator_method() {
    assert_eq!(
        number(
            "class C { *gen() { yield 3; yield 4; } } \
             let it = new C().gen(); it.next().value * 10 + it.next().value;"
        ),
        34.0
    );
}

#[test]
fn class_generator_method_uses_this() {
    assert_eq!(
        number(
            "class C { constructor() { this.v = 9; } *gen() { yield this.v; } } \
             new C().gen().next().value;"
        ),
        9.0
    );
}

#[test]
fn private_generator_method() {
    assert_eq!(
        number(
            "class C { *#gen() { yield 6; } run() { return this.#gen(); } } \
             new C().run().next().value;"
        ),
        6.0
    );
}

#[test]
fn yield_delegation_over_array() {
    assert_eq!(
        number(
            "function* g() { yield* [1, 2, 3]; } \
             let it = g(); let sum = 0; \
             for (let r = it.next(); !r.done; r = it.next()) sum += r.value; sum;"
        ),
        6.0
    );
}

#[test]
fn yield_delegation_over_string() {
    assert_eq!(
        string(
            "function* g() { yield* 'ab'; } \
             let it = g(); it.next().value + it.next().value;"
        ),
        "ab"
    );
}

#[test]
fn yield_delegation_over_generator() {
    assert_eq!(
        number(
            "function* inner() { yield 1; yield 2; } \
             function* outer() { yield* inner(); yield 3; } \
             let it = outer(); \
             it.next().value * 100 + it.next().value * 10 + it.next().value;"
        ),
        123.0
    );
}

#[test]
fn yield_delegation_expression_value_is_inner_return() {
    // `yield* expr` evaluates to the inner iterator's final (done) value.
    assert_eq!(
        number(
            "function* inner() { yield 1; return 42; } \
             function* outer() { let v = yield* inner(); yield v; } \
             let it = outer(); it.next(); it.next().value;"
        ),
        42.0
    );
}

#[test]
fn yield_delegation_nested_three_levels() {
    assert_eq!(
        number(
            "function* a() { yield 1; yield 2; } \
             function* b() { yield* a(); yield 3; } \
             function* c() { yield* b(); yield 4; } \
             let it = c(); let n = 0; \
             for (let r = it.next(); !r.done; r = it.next()) n = n * 10 + r.value; n;"
        ),
        1234.0
    );
}

#[test]
fn yield_delegation_threads_next_argument_into_inner() {
    // `next(v)` while delegating delivers `v` to the inner generator's yield.
    assert_eq!(
        number(
            "function* inner() { let x = yield 1; return x; } \
             function* outer() { let r = yield* inner(); yield r; } \
             let it = outer(); it.next(); it.next(99).value;"
        ),
        99.0
    );
}

#[test]
fn yield_delegation_forwards_throw_into_inner_catch() {
    assert_eq!(
        number(
            "function* inner() { try { yield 1; } catch (e) { yield e + 1; } } \
             function* outer() { yield* inner(); } \
             let it = outer(); it.next(); it.throw(40).value;"
        ),
        41.0
    );
}

#[test]
fn yield_delegation_throw_without_inner_throw_closes_and_type_errors() {
    // A throwless inner iterator is closed (its `return` runs) and the outer
    // `throw` becomes a TypeError at the `yield*` site.
    assert!(boolean(
        "let state = { closed: false }; \
         let iterable = { [Symbol.iterator]() { return this; }, \
             next() { return { value: 1, done: false }; }, \
             return() { state.closed = true; return { value: undefined, done: true }; } }; \
         function* outer() { yield* iterable; } \
         let it = outer(); it.next(); \
         let threw = false; \
         try { it.throw(new TypeError('x')); } catch (e) { threw = true; } \
         state.closed && threw;"
    ));
}

#[test]
fn yield_delegation_forwards_return_and_runs_inner_finally() {
    assert_eq!(
        number(
            "let cleaned = 0; \
             function* inner() { try { yield 1; yield 2; } finally { cleaned = 5; } } \
             function* outer() { yield* inner(); } \
             let it = outer(); it.next(); it.return(0); cleaned;"
        ),
        5.0
    );
}

#[test]
fn yield_delegation_return_without_inner_return_runs_outer_finally() {
    // An inner iterator with no `return` makes `yield*` a return completion,
    // which runs the OUTER generator's enclosing finally.
    assert_eq!(
        number(
            "let cleaned = 0; \
             let iterable = { [Symbol.iterator]() { return this; }, \
                 next() { return { value: 1, done: false }; } }; \
             function* outer() { try { yield* iterable; } finally { cleaned = 8; } } \
             let it = outer(); it.next(); it.return(0); cleaned;"
        ),
        8.0
    );
}

#[test]
fn yield_delegation_return_value_is_returned() {
    // `return(v)` while delegating (inner has no return) completes the outer
    // generator with `{ value: v, done: true }`.
    assert_eq!(
        number(
            "let iterable = { [Symbol.iterator]() { return this; }, \
                 next() { return { value: 1, done: false }; } }; \
             function* outer() { yield* iterable; } \
             let it = outer(); it.next(); it.return(77).value;"
        ),
        77.0
    );
}

#[test]
fn yield_delegation_non_object_inner_result_type_errors() {
    let error = eval(
        "let iterable = { [Symbol.iterator]() { return this; }, \
             next() { return 5; } }; \
         function* outer() { yield* iterable; } \
         let it = outer(); it.next();",
    )
    .unwrap_err();
    assert!(
        error.message.contains("TypeError"),
        "got: {}",
        error.message
    );
}

#[test]
fn yield_delegation_mixed_with_plain_yields() {
    assert_eq!(
        number(
            "function* g() { yield 1; yield* [2, 3]; yield 4; } \
             let it = g(); let n = 0; \
             for (let r = it.next(); !r.done; r = it.next()) n = n * 10 + r.value; n;"
        ),
        1234.0
    );
}

#[test]
fn yield_delegation_over_custom_iterable() {
    assert_eq!(
        number(
            "let iterable = { [Symbol.iterator]() { let i = 0; \
                 return { next() { i++; return { value: i, done: i > 3 }; } }; } }; \
             function* g() { yield* iterable; } \
             let it = g(); let sum = 0; \
             for (let r = it.next(); !r.done; r = it.next()) sum += r.value; sum;"
        ),
        6.0
    );
}

#[test]
fn yield_inside_try_finally_resumes_correctly() {
    // Suspending inside a try block preserves the try/finally stack across the
    // suspension so a later normal completion still runs finally.
    assert_eq!(
        number(
            "let cleaned = 0; \
             function* g() { try { yield 1; yield 2; } finally { cleaned = 7; } } \
             let it = g(); it.next(); it.next(); it.next(); cleaned;"
        ),
        7.0
    );
}

#[test]
fn generator_function_prototype_chain() {
    // A generator function's [[Prototype]] is %GeneratorFunction.prototype%,
    // distinct from %Function.prototype%.
    assert!(boolean(
        "Object.getPrototypeOf(function* () {}) !== Function.prototype;"
    ));
    // All generator functions share the same %GeneratorFunction.prototype%.
    assert!(boolean(
        "Object.getPrototypeOf(function* () {}) === Object.getPrototypeOf(function* () {});"
    ));
    // %GeneratorFunction.prototype%'s [[Prototype]] is %Function.prototype%.
    assert!(boolean(
        "Object.getPrototypeOf(Object.getPrototypeOf(function* () {})) === Function.prototype;"
    ));
    // It carries the GeneratorFunction toStringTag.
    assert_eq!(
        string("Object.prototype.toString.call(Object.getPrototypeOf(function* () {}));"),
        "[object GeneratorFunction]"
    );
    // %GeneratorFunction.prototype%.prototype is the shared %GeneratorPrototype%,
    // which sits in a generator instance's chain.
    assert!(boolean(
        "function* g() {} let gp = Object.getPrototypeOf(g).prototype; gp.isPrototypeOf(g());"
    ));
    // A generator instance still inherits the iterator protocol methods.
    assert_eq!(string("typeof (function* () {})().next;"), "function");
}
