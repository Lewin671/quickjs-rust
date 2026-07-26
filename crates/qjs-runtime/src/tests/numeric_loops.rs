use crate::{Value, eval};

#[test]
fn accumulates_stable_properties_and_dense_indices() {
    assert_eq!(
        eval(
            "function properties(n) { \
               var object = { a: 1, b: 2 }; var sum = 0; \
               for (var i = 0; i < n; i++) { sum += object.a; sum += object.b; } \
               return sum; \
             } \
             function indices(n) { \
               var array = [1, 2, 3]; var sum = 0; \
               for (var i = 0; i < n; i++) { sum += array[0]; sum += array[1]; sum += array[2]; } \
               return sum; \
             } \
             properties(0) + ':' + properties(1) + ':' + properties(5) + ':' + \
               indices(0) + ':' + indices(1) + ':' + indices(5);"
        ),
        Ok(Value::String("0:3:15:0:6:30".to_owned().into()))
    );
}

#[test]
fn accumulates_stable_local_reads_without_freezing_mutating_slots() {
    assert_eq!(
        eval(
            "function stable(n) { \
               var first = 1, second = 2, sum = 0; \
               for (var i = 0; i < n; i++) { sum += first; sum += second; } \
               return sum; \
             } \
             function accumulator(n) { \
               var sum = 1; for (var i = 0; i < n; i++) sum += sum; return sum; \
             } \
             function counter(n) { \
               var sum = 0; for (var i = 0; i < n; i++) sum += i; return sum; \
             } \
             stable(5) + ':' + accumulator(4) + ':' + counter(5);"
        ),
        Ok(Value::String("15:16:10".to_owned().into()))
    );
}

#[test]
fn accumulates_stable_global_reads_but_preserves_global_accessors() {
    assert_eq!(
        eval(
            "var stableValue = 3; \
             function stable(n) { \
               var sum = 0; for (var i = 0; i < n; i++) sum += stableValue; return sum; \
             } \
             var reads = 0; \
             Object.defineProperty(globalThis, 'observedValue', { \
               configurable: true, \
               get: function () { reads += 1; return 2; } \
             }); \
             function observed(n) { \
               var sum = 0; for (var i = 0; i < n; i++) sum += observedValue; return sum; \
             } \
             stable(5) + ':' + observed(4) + ':' + reads;"
        ),
        Ok(Value::String("15:8:4".to_owned().into()))
    );
}

#[test]
fn runs_empty_and_bitwise_branch_control_loops() {
    assert_eq!(
        eval(
            "function empty(n) { var i; for (i = 0; i < n; i++) {} return i; } \
             function branch(n) { \
               var sum = 0; \
               for (var i = 0; i < n; i++) { \
                 if ((i & 1) === 0) sum += 1; else sum += 2; \
               } \
               return sum; \
             } \
             empty(0) + ':' + empty(7) + ':' + branch(0) + ':' + branch(7);"
        ),
        Ok(Value::String("0:7:0:10".to_owned().into()))
    );
}

#[test]
fn falls_back_for_observable_or_non_numeric_reads() {
    assert_eq!(
        eval(
            "var reads = 0; \
             function accessor(n) { \
               var object = { get value() { reads++; return reads; } }; var sum = 0; \
               for (var i = 0; i < n; i++) { sum += object.value; } \
               return sum; \
             } \
             function stringValue(n) { \
               var object = { value: 'x' }; var sum = 0; \
               for (var i = 0; i < n; i++) { sum += object.value; } \
               return sum; \
             } \
             accessor(4) + ':' + reads + ':' + stringValue(3);"
        ),
        Ok(Value::String("10:4:0xxx".to_owned().into()))
    );
}

#[test]
fn falls_back_for_coerced_limits_and_sparse_arrays() {
    assert_eq!(
        eval(
            "function stringLimit(n) { \
               var object = { value: 1 }; var sum = 0; \
               for (var i = 0; i < n; i++) { sum += object.value; } \
               return sum; \
             } \
             function sparse(n) { \
               var array = [, 2]; var sum = 0; \
               for (var i = 0; i < n; i++) { sum += array[0]; } \
               return sum; \
             } \
             stringLimit('3') + ':' + String(sparse(3));"
        ),
        Ok(Value::String("3:NaN".to_owned().into()))
    );
}

#[test]
fn accumulates_numeric_global_local_method_and_stateful_calls() {
    assert_eq!(
        eval(
            "function leaf(x) { return x + 1; } \
             function globalCall(n) { var sum = 0; for (var i = 0; i < n; i++) sum += leaf(i); return sum; } \
             function makeReader() { var captured = 3; return function(x) { return x + captured; }; } \
             function localCall(n) { var f = makeReader(); var sum = 0; for (var i = 0; i < n; i++) sum += f(i); return sum; } \
             function methodCall(n) { var object = { f: function(x) { return x + 2; } }; var sum = 0; for (var i = 0; i < n; i++) sum += object.f(i); return sum; } \
             function makeWriter() { var captured = 0; return function() { captured += 1; return captured; }; } \
             function statefulCall(n) { var f = makeWriter(); var sum = 0; for (var i = 0; i < n; i++) sum += f(); return sum + ':' + f(); } \
             globalCall(6) + ':' + localCall(6) + ':' + methodCall(6) + ':' + statefulCall(6);"
        ),
        Ok(Value::String("21:33:27:21:7".to_owned().into()))
    );
}

#[test]
fn accumulates_two_argument_numeric_global_local_and_method_calls() {
    assert_eq!(
        eval(
            "function add(left, right) { return left + right; } \
             function globalCall(n) { var sum = 0; for (var i = 0; i < n; i++) sum += add(i, 2); return sum; } \
             function localCall(n) { var f = add; var sum = 0; for (var i = 0; i < n; i++) sum += f(i, 3); return sum; } \
             function methodCall(n) { var object = { f: add }; var sum = 0; for (var i = 0; i < n; i++) sum += object.f(i, 4); return sum; } \
             globalCall(4) + ':' + localCall(4) + ':' + methodCall(4);"
        ),
        Ok(Value::String("14:18:22".to_owned().into()))
    );
}

#[test]
fn two_argument_call_loop_trace_falls_back_for_non_numeric_constants() {
    assert_eq!(
        eval(
            "function append(left, right) { return left + right; } \
             function run(n) { var result = ''; for (var i = 0; i < n; i++) result += append(i, 'x'); return result; } \
             run(4);"
        ),
        Ok(Value::String("0x1x2x3x".to_owned().into()))
    );
}

#[test]
fn call_loop_trace_falls_back_for_observable_and_non_numeric_callees() {
    assert_eq!(
        eval(
            "var gets = 0; \
             function accessorCall(n) { \
               var object = { get f() { gets++; return function(x) { return x + 1; }; } }; \
               var sum = 0; for (var i = 0; i < n; i++) sum += object.f(i); return sum; \
             } \
             function booleanCall(n) { \
               var f = function(x) { return x < 2; }; var sum = 0; \
               for (var i = 0; i < n; i++) sum += f(i); return sum; \
             } \
             accessorCall(4) + ':' + gets + ':' + booleanCall(4);"
        ),
        Ok(Value::String("10:4:2".to_owned().into()))
    );
}

#[test]
fn call_loop_trace_rejects_captured_writes_into_the_caller_frame() {
    assert_eq!(
        eval(
            "function shrinkingLimit(n) { \
               var limit = n; \
               var shrink = function() { limit -= 1; return 1; }; \
               var sum = 0; \
               for (var i = 0; i < limit; i++) sum += shrink(); \
               return sum + ':' + limit; \
             } \
             shrinkingLimit(6);"
        ),
        Ok(Value::String("3:3".to_owned().into()))
    );
}

#[test]
fn loop_plan_deoptimization_stays_local_to_one_invocation() {
    // Loop plans live in the shared bytecode and are only copied into a frame
    // when a deoptimization rewrites or suppresses one. A suppression in one
    // call must not leak into later calls of the same function, and repeated
    // calls that alternate between plan-eligible and ineligible inputs must
    // keep producing spec results.
    assert_eq!(
        eval(
            "function accumulate(values, n) { \
               var sum = 0; \
               for (var i = 0; i < n; i++) { sum += values[i]; } \
               return sum; \
             } \
             var numbers = [1, 2, 3, 4]; \
             var mixed = [1, 'a', 3, 4]; \
             var out = []; \
             for (var round = 0; round < 3; round++) { \
               out.push(accumulate(numbers, 4)); \
               out.push(accumulate(mixed, 4)); \
             } \
             out.join(',');"
        ),
        Ok(Value::String("10,1a34,10,1a34,10,1a34".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function mutate(values, n) { \
               for (var i = 0; i < n; i++) { values[i] = values[i] * 2; } \
               return values.join('-'); \
             } \
             var first = [1, 2, 3]; \
             var second = [1, 2, 3]; \
             mutate(first, 3) + '|' + mutate(second, 3) + '|' + mutate(first, 3);"
        ),
        Ok(Value::String("2-4-6|2-4-6|4-8-12".to_owned().into()))
    );
}

#[test]
fn counted_loop_headers_fuse_without_changing_semantics() {
    // The compare-and-branch superinstruction now applies to functions with no
    // virtualizable object literal. It must preserve the loop's observable
    // behavior for zero, one, and many iterations, for a non-numeric operand
    // that leaves the fast path, for a comparison whose operands are captured
    // by a closure, and for a `break`/`continue` that leaves the fused header.
    assert_eq!(
        eval(
            "function run(n) { var s = 0; for (var i = 0; i < n; i++) { s += 2; } return s; } run(0) + ':' + run(1) + ':' + run(5);"
        ),
        Ok(Value::String("0:2:10".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function run(limit) { var s = 0; for (var i = 0; i < limit; i++) { s += 1; } return s; } run('3');"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "function run(n) { var s = 0; for (var i = 0; i < n; i++) { if (i === 2) { continue; } if (i === 4) { break; } s += i; } return s; } run(9);"
        ),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval(
            "function run(n) { var fns = []; for (let i = 0; i < n; i++) { fns.push(function () { return i; }); } return fns.map(function (f) { return f(); }).join(','); } run(3);"
        ),
        Ok(Value::String("0,1,2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function run(n) { var s = 0; var i = 0; while (i < n) { s += i; i = i + 1; } return s + ':' + i; } run(4);"
        ),
        Ok(Value::String("6:4".to_owned().into()))
    );
    // An operand that throws on coercion must still throw from the fused op.
    assert_eq!(
        eval(
            "function run(n) { var s = 0; for (var i = 0; i < n; i++) { s += 1; } return s; } var bad = { valueOf: function () { throw new RangeError('x'); } }; var caught = ''; try { run(bad); } catch (e) { caught = e.constructor.name; } caught;"
        ),
        Ok(Value::String("RangeError".to_owned().into()))
    );
}

#[test]
fn loops_stay_correct_when_a_plan_is_retired_after_repeated_declines() {
    // A numeric loop plan that keeps declining is retired for the rest of the
    // frame, so the loop simply runs on the ordinary interpreter. That is a
    // speed decision with no semantic content, and these loops must produce
    // the same results either way: one that never admits the plan, one that
    // admits it only after several iterations, and one that alternates.
    assert_eq!(
        eval(
            "function helper(v) { return { n: v }; } \
             function run(n) { var s = 0; for (var i = 0; i < n; i++) { s += helper(i).n; } return s; } \
             run(50);"
        ),
        Ok(Value::Number(1225.0))
    );
    assert_eq!(
        eval(
            "function run(n) { var s = 0; var t = 'x'; \
               for (var i = 0; i < n; i++) { if (i === 5) { t = 2; } s += (typeof t === 'number' ? t : 0); } \
               return s; } \
             run(20);"
        ),
        Ok(Value::Number(30.0))
    );
    assert_eq!(
        eval(
            "var values = [1, 2, 3, 4, 5, 6, 7, 8]; \
             function run(n) { var s = 0; \
               for (var i = 0; i < n; i++) { values[i % 8] = i % 2 === 0 ? i : 'skip'; s += (i % 2 === 0 ? i : 0); } \
               return s; } \
             run(40);"
        ),
        Ok(Value::Number(380.0))
    );
    // Repeated invocations must each start with a fresh retry budget.
    assert_eq!(
        eval(
            "function run(n) { var s = 0; for (var i = 0; i < n; i++) { s += i; } return s; } \
             run(10) + ':' + run(10) + ':' + run(100);"
        ),
        Ok(Value::String("45:45:4950".to_owned().into()))
    );
}

#[test]
fn counted_loops_with_literal_bounds_match_local_bound_results() {
    // The literal bound is materialized into a compiler temporary before the
    // loop. The results must be identical to the same loop written with an
    // explicit local bound, including when the body mutates the counter, when
    // the loop body never runs, and when the bound is fractional or negative.
    assert_eq!(
        eval(
            "function run() { var s = 0; for (var i = 0; i < 5; i++) { s += i; } return s + ':' + i; } run();"
        ),
        Ok(Value::String("10:5".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function run() { var s = 0; for (var i = 0; i < 0; i++) { s += 1; } return s + ':' + i; } run();"
        ),
        Ok(Value::String("0:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function run() { var s = 0; for (var i = 0; i < 2.5; i++) { s += 1; } return s; } run();"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "function run() { var s = 0; for (var i = 0; i < -1; i++) { s += 1; } return s; } run();"
        ),
        Ok(Value::Number(0.0))
    );
    // A body that reassigns the counter still terminates against the same
    // bound, and `break`/`continue` are unaffected.
    assert_eq!(
        eval(
            "function run() { var s = 0; for (var i = 0; i < 10; i++) { if (i === 3) { i = 8; continue; } if (i === 9) break; s += i; } return s + ':' + i; } run();"
        ),
        Ok(Value::String("3:9".to_owned().into()))
    );
    // The bound is read once, so a body that shadows or deletes nothing can
    // observe no difference; a `with` scope keeps the unnormalized path.
    assert_eq!(
        eval(
            "var limit = { i: 0 }; var total = 0; with (limit) { for (i = 0; i < 4; i++) { total += i; } } total + ':' + limit.i;"
        ),
        Ok(Value::String("6:4".to_owned().into()))
    );
}
