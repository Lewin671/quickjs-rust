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
fn captured_rest_parameter_shadows_same_named_caller_rest_parameter() {
    assert_eq!(
        eval(
            "function inner(strings, ...subs) {
                 return { toString: () => typeof subs[0] + ':' + subs[0] };
             }
             function outer(strings, ...subs) {
                 return String(subs.map(String));
             }
             var value = inner('', 1);
             outer('', value);"
        ),
        Ok(Value::String("number:1".to_owned().into()))
    );
}

#[test]
fn object_to_string_uses_callee_rest_capture_not_caller_rest_parameter() {
    assert_eq!(
        eval(
            "function render(strings, subs) {
                 return strings.map((str, i) => `${i === 0 ? '' : subs[i - 1]}${str}`).join('');
             }
             function deferred(strings, ...subs) {
                 return { toString: () => render(strings, subs) };
             }
             function outer(...subs) {
                 return String(subs.map(String));
             }
             var value = deferred(['x: ', ''], 1);
             outer([value]);"
        ),
        Ok(Value::String("x: 1".to_owned().into()))
    );
}

#[test]
fn object_to_string_preserves_captured_writeback_side_effects() {
    assert_eq!(
        eval(
            "var value = 0;
             var object = { toString: function() { value = 7; return 'ok'; } };
             String(object);
             value;"
        ),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn constructed_closure_uses_callee_capture_not_caller_same_named_local() {
    assert_eq!(
        eval(
            "function make(name) {
                 return function C() { this.value = name; };
             }
             var C = make('captured');
             function caller(name) {
                 return new C().value;
             }
             caller('caller');"
        ),
        Ok(Value::String("captured".to_owned().into()))
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
fn for_let_initializer_closure_keeps_initial_iteration_environment() {
    assert_eq!(
        eval(
            "var probeBefore, probeTest, probeIncr, probeBody;
             var run = true;
             for (
                 let x = 'outside', _ = probeBefore = function () { return x; };
                 run && (x = 'inside', probeTest = function () { return x; });
                 probeIncr = function () { return x; }
             )
                 probeBody = function () { return x; }, run = false;
             probeBefore() + ':' + probeTest() + ':' + probeBody() + ':' + probeIncr();"
        ),
        Ok(Value::String(
            "outside:inside:inside:inside".to_owned().into()
        ))
    );
}

#[test]
fn for_let_head_blocks_annex_b_function_hoist() {
    assert_eq!(
        eval(
            "(function () {
                for (let f; ; ) {
                    { function f() {} }
                    break;
                }
                try {
                    (function () { f; }());
                } catch (error) {
                    return error.name + ':' + typeof f;
                }
                return 'leaked';
             }());"
        ),
        Ok(Value::String("ReferenceError:undefined".to_owned().into()))
    );
    assert_eq!(
        eval(
            "(function () {
                for (let f; ; ) {
                    { function f() {} }
                    break;
                }
                try {
                    f;
                } catch (error) {
                    return error.name + ':' + typeof f;
                }
                return 'leaked';
             }());"
        ),
        Ok(Value::String("ReferenceError:undefined".to_owned().into()))
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

#[test]
fn forwarded_closure_write_is_visible_to_a_later_read_in_the_same_frame() {
    // A closure invoked through a forwarding frame writes a shared outer
    // binding; a later read in the frame that created the closure must observe
    // that write rather than a stale pre-call snapshot. Affects both a global
    // and a function-local `var`, and the compound/postfix update forms.
    assert_eq!(
        eval(
            "var c = 0; function fwd(f) { f(); } \
             (function () { 'use strict'; fwd(() => { c++; }); c++; })(); c;"
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "function outer() { \
               var c = 0; function fwd(f) { f(); } \
               (function () { 'use strict'; fwd(() => { c += 10; }); c += 1; })(); \
               return c; \
             } outer();"
        ),
        Ok(Value::Number(11.0))
    );
    // The compound-assignment-operator-calls-putvalue idiom: an inner arrow
    // (run via assert.throws-style forwarding) and a sibling update share the
    // same counter.
    assert_eq!(
        eval(
            "var count = 0; function run(f) { f(); } \
             (function () { 'use strict'; run(() => { count += 1; }); count += 1; })(); count;"
        ),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn for_of_let_body_write_reaches_outer_closure() {
    // A closure created BEFORE a `for (let … of …)` loop captures an outer
    // binding. The loop body reassigns that binding each iteration and then
    // invokes the closure (here through a native `Array.prototype` callback).
    // The per-iteration captured env is a fresh snapshot, but writes to the
    // genuinely-outer binding must still reach the pre-loop closure rather than
    // leaving it on the stale `let values;` value (TypedArray/Array
    // resizable-buffer-mid-iteration cluster).
    assert_eq!(
        eval(
            "let values; \
             function cb(n) { values.push(n); return true; } \
             let out = ''; \
             for (let i of [1, 2]) { values = []; [10, 20].every(cb); out += values.join('|') + ';'; } \
             out;"
        ),
        Ok(Value::String("10|20;10|20;".to_owned().into()))
    );
}

#[test]
fn for_of_loop_variable_shadowing_outer_does_not_leak_into_the_outer_binding() {
    // A `for (let x of …)` head whose `x` shadows an outer `let x` must not write
    // the per-iteration value back onto the outer binding — neither its frame
    // slot (read directly after the loop) nor an outer cell a closure created
    // before the loop captured. The leak came from two name-keyed round-trips:
    // `apply_env` after the iterator `next()` call aliased the mangled loop slot
    // under the plain name, and the captured-write propagation skip listed only
    // the mangled storage name.
    assert_eq!(
        eval(
            "function f() { \
               let x = 'outer'; \
               for (let x of ['inner']) {} \
               return x; \
             } f();"
        ),
        Ok(Value::String("outer".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function f() { \
               let x = 'outer'; \
               var probe = function () { return x; }; \
               for (let x of ['inner']) {} \
               return probe(); \
             } f();"
        ),
        Ok(Value::String("outer".to_owned().into()))
    );
    // for-in over an object key set leaks the same way without the fix.
    assert_eq!(
        eval(
            "function f() { \
               let x = 'outer'; \
               for (let x in {a: 1}) {} \
               return x; \
             } f();"
        ),
        Ok(Value::String("outer".to_owned().into()))
    );
}

#[test]
fn call_env_tdz_alias_does_not_clobber_initialized_for_of_binding() {
    // A later closure capture of a same-named `for-of` binding can leave a TDZ
    // alias in a temporary call environment. Writing that environment back must
    // not overwrite the already-initialized binding used by an earlier loop.
    assert_eq!(
        eval(
            "function touch(value) { return String(value); } \
             const cases = [{ label: 'first', args: [] }]; \
             for (const { label, args } of cases) { \
               touch(`seen ${label}`); \
             } \
             if (true) { \
               for (const { label, args } of cases) { \
                 const spy = { toLocaleString(...receivedArgs) { return `later ${label}`; } }; \
                 spy.toLocaleString(...args); \
               } \
             } \
             'ok';"
        ),
        Ok(Value::String("ok".to_owned().into()))
    );
}

#[test]
fn for_of_let_body_write_to_distinct_outer_does_not_disturb_loop_variable() {
    // The per-iteration loop variable keeps its own value while an outer binding
    // written in the same body propagates to a pre-loop closure: the loop walks
    // its own `n` while `total` (outer, captured by `add`) accrues.
    assert_eq!(
        eval(
            "let total; \
             function add(v) { total += v; } \
             let seen = ''; \
             total = 0; \
             for (let n of [1, 2, 3]) { add(n); seen += n; } \
             seen + ':' + total;"
        ),
        Ok(Value::String("123:6".to_owned().into()))
    );
}
