//! Every call owns its activation even when a frame driver or a callback
//! entry rebinds a released one: receiver, new.target, home object, strictness
//! and exposed scopes never leak from the previous callee at that depth.
use crate::{Value, eval};

fn result(source: &str, expected: &str) {
    assert_eq!(eval(source), Ok(Value::String(expected.into())), "{source}");
}

#[test]
fn alternating_methods_keep_their_own_home_objects() {
    result(
        "class A { get v() { return 'a'; } } class B { get v() { return 'b'; } } \
         class C extends A { m() { return super.v; } } class D extends B { m() { return super.v; } } \
         function run(xs) { var out = ''; for (var i = 0; i < xs.length; i++) out += xs[i].m(); return out; } \
         var xs = []; for (var k = 0; k < 60; k++) xs.push(k % 3 ? new C() : new D()); \
         var last; for (var r = 0; r < 4; r++) last = run(xs); last.slice(0, 6)",
        "baabaa",
    );
}

#[test]
fn methods_and_plain_functions_alternate_without_sharing_class_state() {
    result(
        "var o = { base() { return 'p'; }, m() { return super.toString === Object.prototype.toString; } }; \
         Object.setPrototypeOf(o, { toString: Object.prototype.toString }); \
         function plain() { return typeof this; } \
         function run() { var s = ''; for (var i = 0; i < 40; i++) s += (i & 1 ? o.m() : plain.call(5)); return s; } \
         var out; for (var r = 0; r < 4; r++) out = run(); out.slice(0, 20)",
        "objecttrueobjecttrue",
    );
}

#[test]
fn strict_and_sloppy_callees_alternate_with_their_own_receivers() {
    result(
        "function sloppy() { return this === globalThis; } \
         function strict() { 'use strict'; return this === undefined; } \
         function run() { var ok = 0; for (var i = 0; i < 50; i++) ok += (i & 1 ? strict() : sloppy()) ? 1 : 0; return ok; } \
         var out; for (var r = 0; r < 4; r++) out = run(); '' + out",
        "50",
    );
}

#[test]
fn constructor_new_target_does_not_reach_a_later_call() {
    result(
        "function T() { this.t = new.target === T; } function f() { return new.target === undefined; } \
         function run() { var ok = 0; for (var i = 0; i < 50; i++) ok += (i & 1 ? f() : new T().t) ? 1 : 0; return ok; } \
         var out; for (var r = 0; r < 4; r++) out = run(); '' + out",
        "50",
    );
}

#[test]
fn callbacks_that_expose_their_owner_do_not_share_it() {
    result(
        "var arrows = []; \
         function keep(x) { arrows.push(() => this.tag + x); return x; } \
         function plain(x) { return this === undefined || this === globalThis ? x : -1; } \
         var tagged = [1, 2, 3].map(function (x) { return keep.call({ tag: 'k' }, x); }); \
         var sum = [1, 2, 3].reduce(function (a, x) { return a + plain(x); }, 0); \
         arrows.map(f => f()).join() + ':' + sum + ':' + tagged.join()",
        "k1,k2,k3:6:1,2,3",
    );
}

#[test]
fn errors_in_inline_callees_leave_the_caller_receiver_in_place() {
    result(
        "var o = { tag: 'o', bad() { return this.missing.key; }, run() { \
             try { this.bad(); } catch (e) { return this.tag + ':' + e.name; } } }; \
         var out; for (var r = 0; r < 60; r++) out = o.run(); out",
        "o:TypeError",
    );
}

#[cfg(feature = "perf-counters")]
mod tiers {
    //! The cases above run on the register tier's inline frames.
    use crate::{counters, eval, reset_counters};

    #[test]
    fn alternating_methods_run_as_inline_frames() {
        reset_counters();
        eval(
            "class A { constructor() { this.v = 1; } m(o) { var t = this.v; return t + o.v; } } \
             class B { constructor() { this.v = 2; } m(o) { var t = this.v; return t * o.v; } } \
             function run(xs) { var s = 0; for (var i = 0; i < xs.length; i++) s += xs[i].m(xs[0]); return s; } \
             var xs = []; for (var k = 0; k < 60; k++) xs.push(k & 1 ? new A() : new B()); \
             var s; for (var r = 0; r < 4; r++) s = run(xs); if (s !== 210) throw s;",
        )
        .expect("source must evaluate");
        let counters = counters().expect("perf-counters build exposes counters");
        assert!(counters.compact_direct_calls > 0);
    }
}
