//! Global variable stores routed through a frame slot's realm cell or a
//! wide store site's memo stay exact when the realm table changes under them:
//! new names, deletion and re-creation, read-only and accessor redefinition.
use crate::{Value, eval};

fn result(source: &str, expected: &str) {
    assert_eq!(eval(source), Ok(Value::String(expected.into())), "{source}");
}

#[test]
fn slot_routed_global_var_stores_survive_new_globals() {
    result(
        "var a = 0; for (var i = 0; i < 10; i++) { a = a + i; if (i == 4) fresh = 7; } \
         a + ':' + globalThis.a + ':' + fresh;",
        "45:45:7",
    );
}

#[test]
fn eval_declared_globals_deleted_mid_loop_are_recreated() {
    result(
        "var out; (0, eval)(\"var q = 1; for (var j = 0; j < 6; j++) { q = j; \
             if (j == 2) delete globalThis.q; } \
             out = q + ':' + Object.getOwnPropertyDescriptor(globalThis, 'q').configurable;\"); out;",
        "5:true",
    );
}

#[test]
fn memoized_function_stores_follow_deletion_read_only_and_accessors() {
    result(
        "function put(x) { H = x; return x; } \
         var s = 0; for (var k = 0; k < 50; k++) { s += put(k); if (k == 20) delete globalThis.H; } \
         var R = 0; function putR(x) { R = x; return R; } \
         var t = 0; for (var m = 0; m < 30; m++) { t += putR(m); \
             if (m == 10) Object.defineProperty(globalThis, 'R', { writable: false }); } \
         (0, eval)('var acc = 0;'); var seen = 0; function putA(x) { acc = x; } \
         for (var n = 0; n < 30; n++) { putA(n); if (n == 10) Object.defineProperty(globalThis, 'acc', \
             { set: function (v) { seen += v; }, get: function () { return -1; }, configurable: true }); } \
         s + ':' + H + ':' + t + ':' + R + ':' + acc + ':' + seen;",
        "1225:49:245:10:-1:380",
    );
}
