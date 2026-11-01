//! A closure over a scope that sloppy eval can extend resolves each free name
//! through that chain. Indexed tiers may run it without the chain only while
//! the chain provably resolves its free names globally; every declaration,
//! deletion, with object or suspendable owner that could change the answer
//! keeps or returns it to chain resolution, including mid-call.
use crate::{Value, eval};

fn result(source: &str, expected: Value) {
    assert_eq!(eval(source), Ok(expected), "{source}");
}

#[test]
fn eval_declared_names_shadow_and_deletion_unshadows_a_hot_closure() {
    result(
        "function make(n) { var out = 0; var f = function (x) { return Math.abs(x); }; \
             for (var i = 0; i < n; i++) out += f(-i); \
             eval(\"var Math = { abs: function (v) { return 1000; } }\"); \
             for (var i = 0; i < n; i++) out += f(-i); \
             eval(\"delete Math\"); \
             for (var i = 0; i < n; i++) out += f(-i); \
             return out; } \
         make(100);",
        Value::Number(109_900.0),
    );
}

#[test]
fn a_suspended_owner_declaring_mid_call_is_observed() {
    result(
        "var it; function* g() { \
             var f = function () { var a = Math.abs(-1); it.next(); return a + ':' + Math.abs(-1); }; \
             yield f; eval(\"var Math = { abs: function () { return 7; } }\"); yield 0; } \
         it = g(); var f = it.next().value; f();",
        Value::String("1:7".into()),
    );
}

#[test]
fn eval_created_closures_keep_fixed_cells_of_their_free_names() {
    result(
        "function make() { var k = 1; eval(\"var f = function (x) { return Math.abs(x) + k; }\"); \
             return [f, function (v) { k = v; }]; } \
         var p = make(); var s = 0; for (var i = 0; i < 100; i++) s += p[0](-1); \
         p[1](10); s + ':' + p[0](-1);",
        Value::String("200:11".into()),
    );
}

#[test]
fn eval_declarations_shadow_captured_names_between_closure_and_capture() {
    result(
        "function outer(x) { return function () { \
             var get = function () { return x; }; var s = 0; \
             for (var i = 0; i < 100; i++) s += get(); \
             eval('var x = 5'); for (var i = 0; i < 100; i++) s += get(); \
             return s; }; } \
         outer(1)();",
        Value::Number(600.0),
    );
}

#[test]
fn class_constructors_over_eval_scopes_observe_eval_declarations() {
    result(
        "function p() { eval('var Math = { abs: function () { return 7; } }'); \
             class C { constructor() { this.v = Math.abs(-1); } } \
             var r = 0; for (var i = 0; i < 50; i++) r += new C().v; return r; } \
         function q() { eval(''); \
             class C { constructor() { this.f = function () { return Math.abs(-1); }; } } \
             var c; for (var i = 0; i < 50; i++) c = new C(); \
             eval('var Math = { abs: function () { return 7; } }'); return c.f() + new C().f(); } \
         p() + ':' + q();",
        Value::String("350:14".into()),
    );
}

#[test]
fn with_objects_in_the_chain_keep_live_resolution() {
    result(
        "function m(o) { eval(''); with (o) { return function () { return x; }; } } \
         var o = { x: 1 }; var f = m(o); var s = 0; for (var i = 0; i < 100; i++) s += f(); \
         o.x = 5; s + ':' + f();",
        Value::String("100:5".into()),
    );
}

#[test]
fn closures_that_create_closures_keep_their_chain_for_descendants() {
    result(
        "function make() { \
             var outer = function () { return function () { return typeof Math; }; }; \
             var inner; for (var i = 0; i < 100; i++) inner = outer(); \
             var before = inner(); eval(\"var Math = 5\"); return before + ':' + inner(); } \
         make();",
        Value::String("object:number".into()),
    );
}

#[cfg(feature = "perf-counters")]
mod tiers {
    //! Closures whose chain resolves their free names globally take the
    //! indexed entry; a shadowing declaration sends them back.
    use crate::{counters, eval, reset_counters};

    #[test]
    fn closures_over_eval_scopes_take_the_indexed_entry_until_shadowed() {
        reset_counters();
        eval(
            "function make() { var k = 1; eval(''); return function (x) { return Math.abs(x) + k; }; } \
             var f = make(); var s = 0; for (var i = 0; i < 200; i++) s += f(-i); \
             if (s !== 200 + 19900) throw s;",
        )
        .expect("source must evaluate");
        let admitted = counters().expect("perf-counters build exposes counters");
        assert!(
            admitted.direct_leaf_frames >= 200,
            "{}",
            admitted.direct_leaf_frames
        );
        assert_eq!(admitted.lexical_scope_entry_rejections, 0);

        reset_counters();
        eval(
            "function make() { eval('var Math = { abs: function () { return 3; } }'); \
                 return function (x) { return Math.abs(x); }; } \
             var f = make(); var s = 0; for (var i = 0; i < 200; i++) s += f(-i); \
             if (s !== 600) throw s;",
        )
        .expect("source must evaluate");
        let shadowed = counters().expect("perf-counters build exposes counters");
        assert!(shadowed.lexical_scope_entry_rejections > 0);
    }
}
