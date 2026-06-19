use crate::{Value, eval};

#[test]
fn nested_closures_capture_live_outer_bindings() {
    // The activation captured-env snapshot is only materialized when a body
    // creates a closure. These cases keep closure capture correct across the
    // leaf-call fast path: a counter closure must see and mutate its captured
    // binding, and closures created after intervening leaf calls must still
    // capture the current value of an outer binding.
    assert_eq!(
        eval(
            "function make() { var n = 0; return function () { n += 1; return n; }; }
             var inc = make(); inc(); inc(); inc();"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "function leaf(x) { return x + 1; }
             function build() {
                 var total = 0;
                 total += leaf(1);
                 total += leaf(2);
                 return function () { return total; };
             }
             build()();"
        ),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval(
            "function counters() {
                 var fns = [];
                 for (var i = 0; i < 3; i++) {
                     (function (j) { fns.push(function () { return j; }); })(i);
                 }
                 return fns[0]() + ':' + fns[1]() + ':' + fns[2]();
             }
             counters();"
        ),
        Ok(Value::String("0:1:2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function keyed(callback) {
                 combo(function (value) { callback(value); });
             }
             function combo(callback) {
                 callback(1);
             }
             var count = 0;
             keyed(function () { count += 1; });
             count;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "function assertThrows(fn) {
                 try { fn(); } catch (e) { return; }
             }
             function outer() {
                 var last = false;
                 assertThrows(function () {
                     last = 'updated';
                     throw {};
                 });
                 return last;
             }
             outer();"
        ),
        Ok(Value::String("updated".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let src = [42, 43];
             function make() {
                 var src = [1, 2, 3, 4];
                 return function () { return src[0]; };
             }
             let read = make();
             read() + ':' + src.join(',');"
        ),
        Ok(Value::String("1:42,43".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let src = 1;
             function make() {
                 var src = 1;
                 return function () { src = 2; return src; };
             }
             let write = make();
             write() + ':' + src;"
        ),
        Ok(Value::String("2:1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function makeIterable(values) {
                 var src = Array.from(values);
                 var obj = {};
                 obj[Symbol.iterator] = function () { return src[Symbol.iterator](); };
                 return obj;
             }
             let src = [42, 43];
             let sample = new Float64Array(makeIterable([1, 2, 3, 4]));
             sample.set(src, 0);
             src.join(',') + '|' + sample.join(',');"
        ),
        Ok(Value::String("42,43|42,43,3,4".to_owned().into()))
    );
}

#[test]
fn loop_body_lexicals_get_per_iteration_environment() {
    // A `let`/`const`/`class` declared in a `while`/`do`-`while`/`for(;;)` body
    // and captured by a closure must be a fresh binding each iteration, so the
    // closures observe each iteration's value rather than the final one.
    assert_eq!(
        eval(
            "var fns = [];
             var i = 0;
             while (i < 3) { let x = i; fns.push(function () { return x; }); i++; }
             fns.map(function (f) { return f(); }).join(',');"
        ),
        Ok(Value::String("0,1,2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var fns = [];
             var i = 0;
             do { let x = i; fns.push(function () { return x; }); i++; } while (i < 3);
             fns.map(function (f) { return f(); }).join(',');"
        ),
        Ok(Value::String("0,1,2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var fns = [];
             for (var i = 0; i < 3; i++) { let x = i; fns.push(function () { return x; }); }
             fns.map(function (f) { return f(); }).join(',');"
        ),
        Ok(Value::String("0,1,2".to_owned().into()))
    );
    // A lexical declared in a nested block of the body is still per-iteration.
    assert_eq!(
        eval(
            "var fns = [];
             var i = 0;
             while (i < 3) { { let x = i; fns.push(function () { return x; }); } i++; }
             fns.map(function (f) { return f(); }).join(',');"
        ),
        Ok(Value::String("0,1,2".to_owned().into()))
    );
    // `continue` must still pass through the per-iteration refresh.
    assert_eq!(
        eval(
            "var fns = [];
             var i = -1;
             while (i < 3) {
                 i++;
                 if (i === 1) continue;
                 let x = i;
                 fns.push(function () { return x; });
             }
             fns.map(function (f) { return f(); }).join(',');"
        ),
        Ok(Value::String("0,2,3".to_owned().into()))
    );
    // A conditionally-assigned body lexical starts fresh (uninitialized) each
    // iteration rather than carrying the previous iteration's value.
    assert_eq!(
        eval(
            "var fns = [];
             for (var i = 0; i < 3; i++) {
                 let x;
                 if (i === 0) x = 10;
                 fns.push(function () { return x; });
             }
             fns.map(function (f) { return String(f()); }).join(',');"
        ),
        Ok(Value::String("10,undefined,undefined".to_owned().into()))
    );
}

#[test]
fn loop_body_iteration_environment_preserves_outer_captures() {
    // The per-iteration refresh must not freeze captures of a `var` declared
    // *outside* the loop: closures over it still observe the final mutated
    // value rather than a per-iteration snapshot. (Capturing an outer *lexical*
    // binding at script scope has a separate, pre-existing limitation shared by
    // all loop forms — for-of/for-in/for-head included — tracked under T014.)
    assert_eq!(
        eval(
            "var fns = [];
             var s = 0;
             var i = 0;
             while (i < 3) { let x = i; fns.push(function () { return s; }); s++; i++; }
             fns.map(function (f) { return f(); }).join(',');"
        ),
        Ok(Value::String("3,3,3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var fns = [];
             var s = 0;
             for (var i = 0; i < 3; i++) { let x = i; fns.push(function () { return s; }); s += 1; }
             fns.map(function (f) { return f(); }).join(',');"
        ),
        Ok(Value::String("3,3,3".to_owned().into()))
    );
}

#[test]
fn sibling_closure_mutation_observes_latest_var_binding() {
    assert_eq!(
        eval(
            "var c = 9;
             function inc() { c++; return c; }
             function f() { c = 0; var r = inc(); return r + ':' + c; }
             f() + ':' + c;"
        ),
        Ok(Value::String("1:1:1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function outer() {
                 var c = 9;
                 function inc() { c++; }
                 function f() { c = 0; inc(); }
                 f();
                 return c;
             }
             outer();"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "var c = 100;
             function outer() {
                 var c = 9;
                 function inc() { c++; }
                 function f() { c = 0; inc(); return c; }
                 return f() + ':' + c;
             }
             outer() + ':' + c;"
        ),
        Ok(Value::String("1:1:100".to_owned().into()))
    );
}
