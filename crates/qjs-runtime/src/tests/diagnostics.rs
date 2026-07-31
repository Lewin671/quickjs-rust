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
