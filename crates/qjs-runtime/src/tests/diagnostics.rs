//! Execution-counter behavior.
//!
//! These tests exist to keep the counters honest about which tier ran, because
//! that is the only signal that distinguishes an accelerated workload from a
//! folded one. They are gated on `perf-counters` for the same reason the
//! counters are: the default build must not carry them.

#![cfg(feature = "perf-counters")]

use crate::{Value, counters, eval, reset_counters};

fn counted(source: &str) -> (Value, crate::Counters) {
    reset_counters();
    let value = eval(source).expect("source must evaluate");
    (
        value,
        counters().expect("perf-counters build exposes counters"),
    )
}

#[test]
fn recursion_reports_one_call_attempt_per_call() {
    // A binary call tree of depth 4 performs 2**5 - 1 = 31 calls, so a
    // workload that claims 31 calls must report 31 attempts. This is the
    // property that `broad-micro.js` silently violates: its loops fold the
    // callee away and report single-digit attempts for six-figure claims.
    let (value, counters) = counted(
        "function tree(depth, value) {
             if (depth <= 0) { return value + 1; }
             var left = tree(depth - 1, value);
             var right = tree(depth - 1, value);
             return left + right - (value + 1);
         }
         tree(4, 10);",
    );
    assert_eq!(value, Value::Number(11.0));
    assert_eq!(counters.ordinary_call_attempts, 31);
}

#[test]
fn every_call_attempt_is_attributed_to_exactly_one_tier() {
    let (_, counters) = counted(
        "function leaf(value) { return value + 1; }
         function nonLeaf(value) { return leaf(value) + 1; }
         var total = 0;
         for (var i = 0; i < 20; i++) { total += nonLeaf(i); }
         total;",
    );
    let attributed = counters.closed_form_leaf_evaluations
        + counters.direct_leaf_frames
        + counters.generic_call_frames;
    assert_eq!(
        counters.ordinary_call_attempts, attributed,
        "a bytecode call must be counted by exactly one execution tier"
    );
}

#[test]
fn a_folded_loop_reports_far_fewer_calls_than_it_claims() {
    // The shape `broad-micro.js` uses everywhere: a statically resolvable
    // callee in a counted loop. The engine is free to fold it -- the point is
    // that the counter makes the folding visible instead of letting a
    // nanoseconds-per-call figure imply the calls happened.
    let (value, counters) = counted(
        "function addOne(value) { return value + 1; }
         var checksum = 0;
         for (var i = 0; i < 5000; i++) { checksum += addOne(i); }
         checksum;",
    );
    assert_eq!(value, Value::Number(12_502_500.0));
    assert!(
        counters.ordinary_call_attempts < 5000,
        "this shape is expected to fold; if it stops folding, the broad \
         portfolio's numbers changed meaning and its notes need revisiting \
         (attempts: {})",
        counters.ordinary_call_attempts
    );
}

#[test]
fn a_prototype_dispatched_method_is_not_folded() {
    // The sentinel shape. Nothing here may fold: the receiver identity varies
    // and the body reads it.
    let (_, counters) = counted(
        "function Stepper(step) { this.step = step; }
         Stepper.prototype.advance = function (value) { return value + this.step; };
         var pool = [];
         for (var i = 0; i < 4; i++) { pool.push(new Stepper(1)); }
         var checksum = 0;
         for (var j = 0; j < 100; j++) { checksum += pool[j & 3].advance(j); }
         checksum;",
    );
    assert!(
        counters.ordinary_call_attempts >= 100,
        "prototype dispatch over a rotating receiver must run every call \
         (attempts: {})",
        counters.ordinary_call_attempts
    );
}

#[test]
fn an_unrecognized_loop_probes_every_engine_and_enters_none() {
    // Four loop engines are consulted per ordinary backward edge. When none
    // applies, the probes are pure overhead -- this records the ratio the
    // dispatch-table unit would remove.
    let (_, counters) = counted(
        "var seen = [];
         for (var i = 0; i < 50; i++) { seen.push({ index: i }); }
         seen.length;",
    );
    assert!(counters.loop_backedges > 0);
    assert_eq!(counters.loop_plan_entries, 0);
    assert_eq!(counters.declined_loop_plan_edges, counters.loop_backedges);
}

#[test]
fn an_ordinary_call_hands_over_an_environment_that_cannot_carry_markers() {
    // Nothing here can resolve a name dynamically, so every callee gets a
    // freshly built frame and the pre-call marker scrub has nothing to find.
    let (_, counters) = counted(
        "function inner(value) { return value + 1; }
         function outer(value) { return inner(value) + 1; }
         var total = 0;
         for (var i = 0; i < 30; i++) { total += outer(i); }
         total;",
    );
    assert!(counters.ordinary_call_attempts >= 30);
    assert_eq!(counters.call_env_marker_scrubs, 0);
}

#[test]
fn a_callee_handed_the_callers_dynamic_view_still_scrubs() {
    // A Proxy callee is neither a user bytecode function (which would get its
    // own fresh frame) nor a plain native (which resolves through the realm),
    // so a closure-creating caller hands it that caller's own dynamic name
    // view. That environment can carry markers, so the scrub must remain.
    let (_, counters) = counted(
        "var proxied = new Proxy(function (value) { return value; }, {});
         function caller(value) {
             var captured = value;
             var closure = function () { return captured; };
             return proxied(value) + closure();
         }
         var total = 0;
         for (var i = 0; i < 5; i++) { total += caller(i); }
         total;",
    );
    assert!(
        counters.call_env_marker_scrubs > 0,
        "an inherited dynamic frame must keep scrubbing"
    );
}

#[test]
fn direct_eval_still_reaches_caller_locals_across_the_provenance_split() {
    // The marker plumbing is what makes a direct eval resolve names in its
    // caller's frame. Skipping the scrub for provably empty environments must
    // not touch that.
    let (value, _) = counted(
        "function run(start) {
             var local = start;
             eval('local = local + 41;');
             return local;
         }
         run(1);",
    );
    assert_eq!(value, Value::Number(42.0));
}

#[test]
fn an_ordinary_nested_call_does_not_inherit_an_eval_marker() {
    // `plain` resolves `probe` through the global scope. If it inherited the
    // caller's direct-eval view it would see the caller's local instead, so
    // this pins that a nested ordinary call still gets a fresh frame.
    let (value, _) = counted(
        "var probe = 1;
         function plain() { return probe; }
         function withEval() {
             var probe = 100;
             eval('probe = probe + 1;');
             return plain() * 1000 + probe;
         }
         withEval();",
    );
    assert_eq!(value, Value::Number(1101.0));
}
