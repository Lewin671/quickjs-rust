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
        Ok(Value::String("0:1:2".to_owned()))
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
        Ok(Value::String("updated".to_owned()))
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
        Ok(Value::String("1:42,43".to_owned()))
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
        Ok(Value::String("2:1".to_owned()))
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
        Ok(Value::String("42,43|42,43,3,4".to_owned()))
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
        Ok(Value::String("1:1:1".to_owned()))
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
        Ok(Value::String("1:1:100".to_owned()))
    );
}
