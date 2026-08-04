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
fn dispatched_opcode_families_partition_the_generic_loop() {
    let (_, counters) = counted(
        "function calculate(value) {
             var local = value + 1;
             if (local > 2) { local = local * 3; }
             return { result: local }.result;
         }
         calculate(4);",
    );
    let partitioned = counters.dispatched_load_const_ops
        + counters.dispatched_local_binding_ops
        + counters.dispatched_global_binding_ops
        + counters.dispatched_named_property_ops
        + counters.dispatched_computed_property_ops
        + counters.dispatched_call_construct_ops
        + counters.dispatched_stack_ops
        + counters.dispatched_numeric_ops
        + counters.dispatched_branch_return_ops
        + counters.dispatched_general_ops;
    assert!(counters.executed_ops > 0);
    assert_eq!(partitioned, counters.executed_ops);
    assert!(counters.dispatched_local_binding_ops > 0);
    assert_eq!(
        counters.dispatched_load_local_ops
            + counters.dispatched_store_local_ops
            + counters.dispatched_assign_local_ops,
        counters.dispatched_local_binding_ops
    );
    assert!(counters.dispatched_numeric_ops > 0);
    assert!(counters.dispatched_branch_return_ops > 0);
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
        + counters.generic_call_frames
        + counters.compact_direct_calls;
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

#[test]
fn recursion_builds_slot_seeded_frames_and_receiver_arithmetic_builds_none() {
    // This is the frame-stack migration's precondition, measured rather than
    // assumed: the calls it intends to route into one VM must be the ones
    // taking the slot-seeded path, not the general name-keyed prologue.
    //
    // The two halves have since diverged, which is the point of keeping them
    // in one test. Prototype method calls of the `value + this.step` shape
    // left the cohort first -- the closed-form receiver-property tier answers
    // them with no frame and no nested VM. Recursion has now left it too, from
    // the other direction: the compact tier runs an admitted body with no `Vm`
    // at all, so the calls this migration was written to route into one VM no
    // longer construct one to route.
    let (_, recursion) = counted(
        "function tree(depth, value) {
             if (depth <= 0) { return value + 1; }
             var left = tree(depth - 1, value);
             var right = tree(depth - 1, value);
             return left + right - (value + 1);
         }
         tree(5, 3);",
    );
    assert_eq!(recursion.ordinary_call_attempts, 63);
    // Only the outermost call still builds a slot-seeded frame, because only
    // it is reached from the ordinary interpreter. The other 62 are compact
    // bodies calling a compact body: borrowed environment, register file, no
    // frame and no environment construction at all.
    assert_eq!(recursion.direct_leaf_frames, 1);
    assert_eq!(recursion.compact_direct_calls, 62);
    assert_eq!(recursion.generic_call_frames, 0);
    // Every one of the 63 calls now runs on a compact activation, and the only
    // nested VM left is the top-level script's. This assertion used to read
    // `nested_vm_constructions >= 63` -- one per call, the cost the migration
    // was written to remove. It is asserted exactly, rather than as a bound,
    // so that a body silently falling back to the ordinary frame fails here.
    assert_eq!(recursion.compact_standalone_activations, 63);
    assert_eq!(recursion.nested_vm_constructions, 1);
    assert_eq!(recursion.same_vm_frame_entries, 0);

    let (_, methods) = counted(
        "function Stepper(step) { this.step = step; }
         Stepper.prototype.advance = function (value) { return value + this.step; };
         var pool = [];
         for (var i = 0; i < 4; i++) { pool.push(new Stepper(1)); }
         var checksum = 0;
         for (var j = 0; j < 40; j++) { checksum += pool[j & 3].advance(j); }
         checksum;",
    );
    // These forty calls used to build forty slot-seeded frames. They now build
    // none: `value + this.step` is answered by the closed-form
    // receiver-property tier, which is the outcome the frame migration was
    // meant to approximate for this shape and reaches it without a frame at
    // all. The counters say so precisely, which is why the assertion is on the
    // tier rather than on wall time.
    assert_eq!(methods.closed_form_leaf_evaluations, 40);
    assert_eq!(methods.direct_leaf_frames, 0);
    // The four `new Stepper(1)` constructions are the only general frames:
    // `Stepper` is an ordinary function, so constructing it is outside both
    // slot-seeding predicates.
    assert_eq!(methods.generic_call_frames, 4);
    // Five nested VMs remain: the four constructions and the top-level script.
    // The method calls contribute none.
    assert_eq!(methods.nested_vm_constructions, 5);
}

#[test]
fn supplying_loop_plans_externally_does_not_change_which_plan_claims_a_site() {
    // The loop accelerators are now handed to dispatch instead of read off the
    // frame, so the frame can stop borrowing its bytecode. That is a plumbing
    // change only: if it altered which engine claims a backward edge, or how
    // many edges are consulted, it would have changed execution rather than
    // representation.
    let cases: &[(&str, u64, u64)] = &[
        // A counted numeric loop in a function body: one engine claims the
        // whole region, so the frame takes a single backward edge.
        (
            "function run() {
                 var total = 0;
                 for (var i = 0; i < 200; i++) { total += i; }
                 return total;
             }
             run();",
            1,
            0,
        ),
        // The same loop at global scope, where the counters are realm
        // bindings: no engine claims it, and every edge consults all four.
        (
            "var total = 0;
             for (var i = 0; i < 200; i++) { total += i; }
             total;",
            0,
            200,
        ),
        // A loop whose body allocates: also unclaimed, one declined chain per
        // iteration.
        (
            "var seen = [];
             for (var i = 0; i < 60; i++) { seen.push({ index: i }); }
             seen.length;",
            0,
            60,
        ),
    ];
    for (source, expected_entries, expected_declined) in cases {
        let (_, counters) = counted(source);
        assert_eq!(counters.loop_plan_entries, *expected_entries, "{source}");
        assert_eq!(
            counters.declined_loop_plan_edges, *expected_declined,
            "{source}"
        );
        assert_eq!(
            counters.loop_backedges,
            counters.loop_plan_entries + counters.declined_loop_plan_edges,
            "every backward edge either enters a plan or declines every plan"
        );
    }
}
