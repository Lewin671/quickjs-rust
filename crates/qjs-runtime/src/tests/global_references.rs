//! A global assignment keeps the resolvability its reference had before the
//! RHS (ES2025 13.15.2, 9.1.1.4.5), on every tier. A bound global reference
//! and a late resolution agree; an originally unresolvable one must not be
//! re-resolved after the RHS deletes, creates or declares the name. Each case
//! runs at top level, once in a function, and in a function hot enough for
//! the register tier, with the effect on its last call.
use crate::{Value, eval};

fn result(source: &str, expected: &str) {
    assert_eq!(eval(source), Ok(Value::String(expected.into())), "{source}");
}

const HELPERS: &str = "var ES = __quickjsRustEvalScript; \
    function del(name) { delete globalThis[name]; } \
    function hot(f) { for (var i = 0; i <= 50; i++) f(i, i === 50); }";

fn with_helpers(body: &str, expected: &str) {
    result(&format!("{HELPERS} {body}"), expected);
}

#[test]
fn global_lexical_declared_by_the_rhs_of_an_unresolvable_sloppy_name() {
    // The reference was unresolvable: PutValue sets the global object's
    // property and the new lexical keeps its own value.
    with_helpers(
        "top1 = (ES('let top1 = 0;'), 5); globalThis.top1 + ':' + top1",
        "5:0",
    );
    with_helpers(
        "function f() { fn1 = (ES('let fn1 = 0;'), 5); } f(); globalThis.fn1 + ':' + fn1",
        "5:0",
    );
    with_helpers(
        "hot(function (i, last) { if (last) del('hot1'); hot1 = (last ? ES('let hot1 = 0;') : 0, i); }); \
         globalThis.hot1 + ':' + hot1",
        "50:0",
    );
}

#[test]
fn global_lexical_declared_by_the_rhs_of_a_bound_name() {
    // A bound global reference consults the declarative record first.
    with_helpers(
        "top2 = 1; top2 = (ES('let top2 = 0;'), 5); globalThis.top2 + ':' + top2",
        "1:5",
    );
    with_helpers(
        "hot(function (i, last) { hot2 = (last ? ES('let hot2 = 0;') : 0, i); }); \
         globalThis.hot2 + ':' + hot2",
        "49:50",
    );
}

#[test]
fn rhs_deletion_and_recreation_of_a_bound_name() {
    with_helpers(
        "hot(function (i, last) { hot3 = (last ? del('hot3') : 0, i); }); \
         var d = Object.getOwnPropertyDescriptor(globalThis, 'hot3'); d.value + ':' + d.configurable",
        "50:true",
    );
    with_helpers(
        "globalThis.hot4 = 0; var seen = 'none'; \
         try { hot(function (i, last) { 'use strict'; hot4 = (last ? del('hot4') : 0, i); }); } \
         catch (e) { seen = e.name; } seen + ':' + ('hot4' in globalThis)",
        "ReferenceError:false",
    );
    with_helpers(
        "globalThis.hot5 = 0; \
         hot(function (i, last) { 'use strict'; hot5 = (last ? (del('hot5'), globalThis.hot5 = -1) : 0, i); }); \
         '' + hot5",
        "50",
    );
}

#[test]
fn deletion_and_recreation_before_a_callback_declares_the_lexical() {
    // Bound before the RHS: the declarative record wins at PutValue.
    with_helpers(
        "hot(function (i, last) { \
             hot6 = (last ? (del('hot6'), globalThis.hot6 = -1, ES('let hot6 = 0;')) : 0, i); }); \
         globalThis.hot6 + ':' + hot6",
        "-1:50",
    );
    // Unresolvable before the RHS: the property recreated by the RHS is the
    // target even though a same-named lexical now shadows it.
    with_helpers(
        "hot(function (i, last) { if (last) del('hot7'); \
             hot7 = (last ? (globalThis.hot7 = -1, ES('let hot7 = 0;')) : 0, i); }); \
         globalThis.hot7 + ':' + hot7",
        "50:0",
    );
}

#[test]
fn strict_missing_names_stay_unresolvable() {
    with_helpers(
        "globalThis.hot8 = 0; var seen = 'none'; \
         try { hot(function (i, last) { 'use strict'; if (last) del('hot8'); \
             hot8 = (last ? (globalThis.hot8 = -1) : 0, i); }); } \
         catch (e) { seen = e.name; } seen + ':' + globalThis.hot8",
        "ReferenceError:-1",
    );
    with_helpers(
        "globalThis.hot9 = 0; var seen = 'none'; \
         try { hot(function (i, last) { 'use strict'; if (last) del('hot9'); \
             hot9 = (last ? ES('let hot9 = 7;') : 0, i); }); } \
         catch (e) { seen = e.name; } seen + ':' + hot9 + ':' + ('hot9' in globalThis)",
        "ReferenceError:7:false",
    );
}

#[test]
fn eval_declared_globals_are_configurable_bound_names() {
    with_helpers(
        "eval('var ev1 = 1;'); hot(function (i, last) { ev1 = (last ? del('ev1') : 0, i); }); \
         Object.getOwnPropertyDescriptor(globalThis, 'ev1').configurable + ':' + ev1",
        "true:50",
    );
    with_helpers(
        "(0, eval)('var ev2 = 1;'); var seen = 'none'; \
         try { hot(function (i, last) { 'use strict'; ev2 = (last ? del('ev2') : 0, i); }); } \
         catch (e) { seen = e.name; } seen",
        "ReferenceError",
    );
}

#[test]
fn sloppy_store_after_deletion_ignores_the_stale_captured_cell() {
    // The function's captured realm cell for `y` is invalidated by the
    // deletion; the elided store must write the recreated binding instead.
    with_helpers(
        "y1 = 0; function f(v) { y1 = v; return y1; } f(1); del('y1'); y1 = 2; \
         f(3) + ':' + y1 + ':' + Object.getOwnPropertyDescriptor(globalThis, 'y1').value",
        "3:3:3",
    );
    with_helpers(
        "y2 = 0; function g(v) { y2 = v; return y2; } g(1); var hits = 0; \
         Object.defineProperty(globalThis, 'y2', { set(v) { hits += v; }, get() { return hits; }, configurable: true }); \
         g(5) + ':' + hits",
        "5:5",
    );
}

#[cfg(feature = "perf-counters")]
mod tiers {
    //! Configurable global names no longer push their bodies off the
    //! accelerated tiers; only an unresolvable start is selected.
    use crate::{counters, eval, reset_counters};

    fn counted(source: &str) -> crate::Counters {
        reset_counters();
        eval(source).expect("source must evaluate");
        counters().expect("perf-counters build exposes counters")
    }

    #[test]
    fn typed_loops_claim_implicit_global_stores() {
        let counters = counted(
            "g = 4294967296; for (var i = 0; i < 2000; i++) g = g & i; if (g !== 0) throw g;",
        );
        assert_eq!(counters.reference_blocked_loop_edges, 0);
        assert!(counters.loop_plan_entries > 0);
    }

    #[test]
    fn typed_loop_global_sink_is_admitted_on_later_calls() {
        let counters = counted(
            "function f(n) { var a = 0; for (var k = 1; k <= n; k++) { s = k * 0.5; a += k; } return a; } \
             for (var j = 0; j < 5; j++) f(200);",
        );
        assert_eq!(counters.reference_blocked_loop_edges, 0);
        assert!(
            counters.loop_plan_entries >= 4,
            "{}",
            counters.loop_plan_entries
        );
    }

    #[test]
    fn register_tier_selects_only_the_unresolvable_start() {
        let counters = counted(
            "function mk(n) { var t = ''; for (var i = 0; i < n; i++) t += 'a'; return t; } \
             function run() { for (var k = 0; k < 200; k++) { user1 = mk(3); mail1 = user1 + '@x'; } } \
             run(); if (mail1 !== 'aaa@x') throw mail1;",
        );
        assert!(counters.compact_function_ops > 0);
        // The first iteration's two starts are unresolvable; every later
        // start proves its now-bound property from the cached proof.
        assert_eq!(counters.reference_checkpoint_selections, 2);
    }
}
