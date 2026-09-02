//! Frame state that ordinary calls never touch -- `try`/`finally`
//! bookkeeping, `with` scopes, generator resumes, sloppy global names -- lives
//! behind a lazily materialized, pooled box. These pin that a body which does
//! use it sees a clean box on every activation, including after a throw left
//! the previous activation's box non-empty, and across recursion, where
//! several activations of one body hold boxes at once.

use crate::{Value, eval};

fn value_of(source: &str) -> Value {
    eval(source).expect("source must evaluate")
}

#[test]
fn a_pooled_cold_frame_is_clean_after_a_throwing_activation() {
    assert_eq!(
        value_of(
            "var finals = 0;
             function f(i) {
               try {
                 if (i === 0) throw new Error('x');
                 return i;
               } catch (e) {
                 return -1;
               } finally {
                 finals++;
               }
             }
             var r = [f(0), f(1), f(2)];
             r[0] === -1 && r[1] === 1 && r[2] === 2 && finals === 3;"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn finally_overrides_and_deferred_jumps_survive_repeated_activations() {
    assert_eq!(
        value_of("function f() { try { return 1; } finally { return 2; } } f() + f();"),
        Value::Number(4.0)
    );
    assert_eq!(
        value_of(
            "function f() {
               var n = 0;
               for (var i = 0; i < 3; i++) {
                 try { if (i === 1) break; n++; } finally { n += 10; }
               }
               return n;
             }
             f() + f();"
        ),
        Value::Number(42.0)
    );
}

#[test]
fn recursion_holds_one_cold_frame_per_live_activation() {
    assert_eq!(
        value_of(
            "function f(n) {
               try {
                 if (n > 0) return f(n - 1) + 1;
                 throw 0;
               } catch (e) {
                 return 100;
               }
             }
             f(5);"
        ),
        Value::Number(105.0)
    );
}

#[test]
fn with_scopes_are_private_to_each_activation() {
    assert_eq!(
        value_of("function f(o) { with (o) { return v; } } f({ v: 1 }) + f({ v: 2 });"),
        Value::Number(3.0)
    );
}

#[test]
fn a_generator_carries_its_try_state_across_suspension() {
    assert_eq!(
        value_of(
            "var log = [];
             function* g() {
               try { yield 1; yield 2; } finally { log.push('f'); }
             }
             var it = g();
             it.next();
             it.return(9);
             var it2 = g();
             it2.next(); it2.next(); it2.next();
             log.join(',') === 'f,f';"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn sloppy_global_names_are_recorded_per_activation() {
    assert_eq!(
        value_of(
            "function f() { undeclaredSloppy = 1; return typeof undeclaredSloppy; }
             f(); f();
             typeof undeclaredSloppy === 'number';"
        ),
        Value::Boolean(true)
    );
}
