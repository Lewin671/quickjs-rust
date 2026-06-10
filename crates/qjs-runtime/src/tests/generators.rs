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
fn yield_delegation_is_not_yet_supported() {
    // `yield*` lands in T010 S3; it must report a structured early error.
    let error = eval("function* g() { yield* [1, 2]; } g();").unwrap_err();
    assert!(error.message.contains("yield*"), "got: {}", error.message);
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
