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
fn empty_and_bitwise_branch_counted_loops_keep_their_results() {
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
fn accumulation_loops_observe_accessors_and_concatenate_string_reads() {
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
fn accumulation_loops_coerce_string_limits_and_read_holes_as_undefined() {
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
fn two_argument_call_loops_concatenate_string_arguments() {
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
fn call_loops_observe_accessor_callees_and_add_boolean_results() {
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
fn call_loops_see_callee_writes_to_a_captured_loop_limit() {
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
fn dense_array_loops_keep_results_when_calls_alternate_numeric_and_mixed_inputs() {
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
fn counted_loop_headers_preserve_coercion_break_continue_and_capture_semantics() {
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
fn loops_with_allocating_type_changing_or_mixed_store_bodies_keep_their_results() {
    // A loop plan that keeps declining is retired for the rest of the
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

#[test]
fn brace_less_loop_bodies_match_braced_ones() {
    // A brace-less body is one statement, not a statement list, and used to
    // keep its value in the loop's completion temporary, which made a string
    // append quadratic.
    assert_eq!(
        eval("function m(){ var s = ''; for (var i = 0; i < 4; i++) s += 'ab'; return s; } m();"),
        Ok(Value::String("abababab".to_owned().into()))
    );
    assert_eq!(
        eval("function m(){ var s = 0; for (var i = 0; i < 5; i++) s += i; return s; } m();"),
        Ok(Value::Number(10.0))
    );
    // The loop's own completion value is still observable at script level.
    assert_eq!(
        eval("var t = 0; for (var i = 0; i < 3; i++) t += i;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("eval('var u = 0; for (var i = 0; i < 3; i++) u += i;');"),
        Ok(Value::Number(3.0))
    );
    // `break`, `continue`, and a nested brace-less body keep working.
    assert_eq!(
        eval(
            "function m(){ var s = ''; outer: for (var i = 0; i < 4; i++) for (var j = 0; j < 4; j++) { if (j > i) continue outer; s += i + '' + j; } return s; } m();"
        ),
        Ok(Value::String("00101120212230313233".to_owned().into()))
    );
}

/// Numeric helpers that call helpers -- self and mutual recursion, a
/// callee returning a boolean or sometimes `undefined`, recursion past the
/// native bound, a boolean argument -- give the interpreter's answers from
/// a typed loop. Expected values from V8.
#[test]
fn recursive_numeric_helpers_called_from_typed_loops() {
    let source = r#"function tree(d, v) { if (d <= 0) return v + 1; return tree(d - 1, v) + tree(d - 1, v) - (v + 1); }
function fib(n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }
function isEven(n) { return n === 0 ? 1 : isOdd(n - 1); }
function isOdd(n) { return n === 0 ? 0 : isEven(n - 1); }
function deep(n) { return n <= 0 ? 0 : 1 + deep(n - 1); }
function pos(x) { return x > 0; }
function countPos(n) { return n <= 0 ? 0 : (pos(n) ? 1 : 0) + countPos(n - 1); }
function maybe(n) { if (n > 3) return n; }
function sumMaybe(n) { var m = maybe(n); return m === undefined ? -1 : m; }
function half(n) { return n <= 1 ? n : half(n / 2); }
var out = [];
var s = 0; for (var i = 0; i < 50; i++) s += tree(5, i); out.push(s);
s = 0; for (var i = 0; i < 20; i++) s += fib(i); out.push(s);
s = 0; for (var i = 0; i < 30; i++) s += isEven(i); out.push(s);
s = 0; for (var i = 0; i < 5; i++) s += deep(200 + i); out.push(s);
s = 0; for (var i = 0; i < 10; i++) s += countPos(i); out.push(s);
s = 0; for (var i = 0; i < 8; i++) s += sumMaybe(i); out.push(s);
s = 0; for (var i = 0; i < 8; i++) s += half(i * 3); out.push(s);
s = 0; for (var i = 0; i < 5; i++) s += fib(i % 2 ? true : 3); out.push(s);
out.join(',');"#;
    assert_eq!(
        eval(source),
        Ok(Value::String(
            "1275,10945,15,1010,45,18,4.96875,8".to_owned().into()
        ))
    );
}

/// A number-only helper takes a boolean or `undefined` argument by
/// `ToNumber` when every parameter reaches its result through an operator
/// (`safe_add(s, w[j])` past the end of `w`), and never when one is
/// returned as passed (`id`, `keep`). Expected values from V8.
#[test]
fn number_only_helpers_convert_arguments_only_through_operators() {
    let source = r#"function safe_add(x, y) { var lsw = (x & 0xFFFF) + (y & 0xFFFF); var msw = (x >> 16) + (y >> 16) + (lsw >> 16); return (msw << 16) | (lsw & 0xFFFF); }
function id(x) { return x; }
function keep(x) { var y = x; return y; }
function plus(x, y) { return x + y; }
function run(n) {
  var w = [1, 2, 3], out = [], s = 0;
  for (var j = 0; j < n; j++) { s = safe_add(s, w[j]); }
  out.push(s);
  var r = []; for (var j = 0; j < 4; j++) r.push(String(id(w[j])));
  out.push(r.join('/'));
  r = []; for (var j = 0; j < 4; j++) r.push(String(keep(w[j])));
  out.push(r.join('/'));
  r = []; for (var j = 0; j < 4; j++) r.push(String(plus(w[j], 1)));
  out.push(r.join('/'));
  var b = [true, false, undefined, 2]; s = 0;
  for (var j = 0; j < 4; j++) s = safe_add(s, b[j]); out.push(s);
  return out.join(',');
}
run(6);"#;
    assert_eq!(
        eval(source),
        Ok(Value::String(
            "6,1/2/3/undefined,1/2/3/undefined,2/3/4/NaN,3"
                .to_owned()
                .into()
        ))
    );
}

#[test]
fn conditional_value_stored_in_a_loop_is_written_on_both_arms() {
    let source = r#"
function plain(n) {
  var first = 2, second = 5, selected, sum = 0;
  for (var i = 0; i < n; i++) { selected = (i & 1) === 0 ? first : second; sum += selected; }
  return sum;
}
function declared(n) {
  var a = 2, b = 5, s = 0;
  for (var i = 0; i < n; i++) { var x = (i & 1) ? a : b; s += x * i; }
  return s;
}
function subtracted(n) {
  var a = 2, b = 5, x, s = 0;
  for (var i = 0; i < n; i++) { x = (i & 1) ? a : b; s = s - x; }
  return s;
}
function computed(n) {
  var first = 2, second = 5, selected, sum = 0;
  for (var i = 0; i < n; i++) { selected = (i & 1) === 0 ? first + 1 : second + 1; sum += selected; }
  return sum;
}
[plain(6), declared(6), subtracted(6), computed(6), plain(0), declared(1)].join(' ');"#;
    assert_eq!(
        eval(source),
        Ok(Value::String("21 48 -21 27 0 0".to_owned().into()))
    );
}

/// Counted loops whose whole body is one bitwise recurrence on a scalar
/// (`value = value <op> operand`): every operator, the ToInt32 / shift-count
/// boundaries, counter wrap past 2^32, and the final counter value.
#[test]
fn counted_loops_updating_one_scalar_with_a_bitwise_operator_match_int32_semantics() {
    for (source, expected) in [
        (
            "function run(value) { for (var i = 1; i < 5; i++) value = value & i; return value + ':' + i; } run(-1);",
            "0:5",
        ),
        (
            "function run(value) { for (var i = 1; i < 5; i++) value = value | i; return value + ':' + i; } run(0);",
            "7:5",
        ),
        (
            "function run(value) { for (var i = 1; i < 5; i++) value = value ^ i; return value + ':' + i; } run(0);",
            "4:5",
        ),
        (
            "function run(value) { for (var i = 0; i < 3; i++) value = value << i; return value + ':' + i; } run(1);",
            "8:3",
        ),
        (
            "function run(value) { for (var i = 0; i < 3; i++) value = value >> 1; return value + ':' + i; } run(-16);",
            "-2:3",
        ),
        (
            "function run(value) { for (var i = 0; i < 2; i++) value = value >>> 1; return value + ':' + i; } run(-1);",
            "1073741823:2",
        ),
        (
            "function run(value) { for (var i = 4294967294; i < 4294967298; i++) value = value ^ i; return value + ':' + i; } run(0);",
            "0:4294967298",
        ),
        (
            "function run(limit, value) { for (var i = -2; i < limit; i++) value ^= i; return Object.is(i, -0) + ':' + value; } run(-0, 0);",
            "false:1",
        ),
        (
            "function scramble(rounds, state, mask) { for (var cursor = 0; cursor < rounds; cursor++) state ^= mask; return state + ':' + cursor; } scramble(17, 123, 42);",
            "81:17",
        ),
    ] {
        assert_eq!(
            eval(source),
            Ok(Value::String(expected.to_owned().into())),
            "{source}"
        );
    }
    for (source, expected) in [
        (
            "function run(value, rhs) { for (var i = 0; i < 3; i++) value = value | rhs; return value; } run(NaN, Infinity);",
            Value::Number(0.0),
        ),
        (
            "function run(value, rhs) { for (var i = 0; i < 3; i++) value = value ^ rhs; return Object.is(value, -0); } run(-0, -0);",
            Value::Boolean(false),
        ),
        (
            "function run(value) { for (var i = 0; i < 3; i++) value = value >>> 0; return value; } run(-1);",
            Value::Number(4_294_967_295.0),
        ),
        (
            "function run(value) { for (var i = 0; i < 3; i++) value = value << 33; return value; } run(1);",
            Value::Number(8.0),
        ),
        (
            "function run(value, rhs) { for (var i = 0; i < 2; i++) value = value << rhs; return value; } run(1, -1);",
            Value::Number(0.0),
        ),
        (
            "function run(value) { for (var i = 0; i < 3; i++) value = value << 32; return value; } run(1);",
            Value::Number(1.0),
        ),
        (
            "function run(value) { for (var i = 0; i < 3; i++) value = value >> 1.9; return value; } run(8);",
            Value::Number(1.0),
        ),
    ] {
        assert_eq!(eval(source), Ok(expected), "{source}");
    }
}

#[test]
fn bitwise_update_loop_on_an_undeclared_sloppy_global_updates_the_global() {
    assert_eq!(
        eval(
            "bitwiseValue = 4294967296; \
             for (var arbitraryCounter = 0; arbitraryCounter < 600000; arbitraryCounter++) \
               bitwiseValue = bitwiseValue & arbitraryCounter; \
             bitwiseValue + ':' + arbitraryCounter + ':' + globalThis.bitwiseValue;"
        ),
        Ok(Value::String("0:600000:0".to_owned().into()))
    );
}

#[test]
fn bitwise_update_loops_coerce_objects_each_iteration_and_keep_bigint_rules() {
    assert_eq!(
        eval(
            "var coercions = 0; var operand = { valueOf: function () { coercions++; return 3; } }; \
             function run(value) { for (var i = 0; i < 4; i++) value = value & operand; return value + ':' + i; } \
             run(7) + ':' + coercions;"
        ),
        Ok(Value::String("3:4:4".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function run(value, rhs) { for (var i = 0; i < 4; i++) value = value & rhs; return value === 1n; } run(5n, 3n);"
        ),
        Ok(Value::Boolean(true))
    );
    assert!(eval(
        "function run(value, rhs) { for (var i = 0; i < 4; i++) value = value & rhs; return value; } run(5, 3n);"
    )
    .is_err());
    assert!(eval(
        "function run(value, rhs) { for (var i = 0; i < 4; i++) value = value >>> rhs; return value; } run(5n, 1n);"
    )
    .is_err());
}

#[test]
fn bitwise_update_loops_respect_accessors_readonly_globals_eval_and_captures() {
    assert_eq!(
        eval(
            "guardedValue = 7; var gets = 0; \
             Object.defineProperty(globalThis, 'guardedValue', { configurable: true, \
               get: function () { gets++; return 7; } }); \
             for (var i = 0; i < 4; i++) guardedValue = guardedValue & i; \
             gets + ':' + i;"
        ),
        Ok(Value::String("4:4".to_owned().into()))
    );
    assert_eq!(
        eval(
            "readOnlyValue = 7; Object.defineProperty(globalThis, 'readOnlyValue', { writable: false }); \
             for (var i = 0; i < 4; i++) readOnlyValue = readOnlyValue & i; \
             readOnlyValue + ':' + i;"
        ),
        Ok(Value::String("7:4".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function run(value) { eval('value = value'); for (var i = 0; i < 4; i++) value = value ^ i; return value; } run(0);"
        ),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "function run(value) { function read() { return value; } for (var i = 0; i < 4; i++) value = value | i; return value + read(); } run(0);"
        ),
        Ok(Value::Number(6.0))
    );
}

#[test]
fn bitwise_update_loops_handle_fractional_limits_and_single_iterations() {
    assert_eq!(
        eval(
            "function run(limit, value) { for (var i = 0; i < limit; i++) value = value ^ i; return value + ':' + i; } run(3.5, 0);"
        ),
        Ok(Value::String("0:4".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function run(value) { for (var i = 0; i < 1; i++) value = value | i; return value + ':' + i; } run(3);"
        ),
        Ok(Value::String("3:1".to_owned().into()))
    );
}

// The tests below ran through the removed numeric loop plan (a counted loop
// accumulating stable reads or numeric leaf calls into one variable). They
// keep its JavaScript-visible contract on whichever tier now runs the loop.

fn assert_string(source: &str, expected: &str) {
    assert_eq!(
        eval(source),
        Ok(Value::String(expected.to_owned().into())),
        "{source}"
    );
}

fn assert_number(source: &str, expected: f64) {
    assert_eq!(eval(source), Ok(Value::Number(expected)), "{source}");
}

#[test]
fn top_level_var_accumulation_loops_update_global_bindings() {
    assert_string(
        "function addOne(value) { return value + 1; } \
         var limit = 1000; var checksum = 0; \
         for (var index = 0; index < limit; index++) checksum += addOne(index); \
         checksum + ':' + index + ':' + globalThis.checksum + ':' + \
           Object.getOwnPropertyNames(globalThis).filter(function (name) { \
             return name.length >= 2 && name.charCodeAt(0) === 0 && name.charCodeAt(1) === 0; \
           }).length;",
        "500500:1000:500500:0",
    );
    assert_string(
        "function addOne(value) { return value + 1; } var limit = 4, sum = 0; \
         Object.defineProperty(globalThis, 'sum', { value: 5 }); \
         for (var index = 0; index < limit; index++) sum += addOne(index); \
         sum + ':' + globalThis.sum;",
        "15:15",
    );
}

#[test]
fn top_level_accumulation_loops_observe_aliased_global_reads() {
    assert_string(
        "var limit = 4, sum = 0; function addCurrent(value) { return value + sum; } \
         for (var index = 0; index < limit; index++) sum += addCurrent(index); \
         sum + ':' + index;",
        "11:4",
    );
    for source in [
        "var mirror = globalThis, limit = 4, sum = 0; for (var index = 0; index < limit; index++) sum += mirror.index; sum + ':' + index;",
        "var mirror = globalThis, key = 'index', limit = 4, sum = 0; for (var index = 0; index < limit; index++) sum += mirror[key]; sum + ':' + index;",
    ] {
        assert_string(source, "6:4");
    }
}

#[test]
fn top_level_accumulation_loops_respect_descriptors_and_dynamic_scopes() {
    for (source, expected) in [
        (
            "function addOne(value) { return value + 1; } var limit = 4, sum = 0; Object.defineProperty(globalThis, 'sum', { writable: false }); for (var index = 0; index < limit; index++) sum += addOne(index); sum + ':' + index;",
            "0:4",
        ),
        (
            "function addOne(value) { return value + 1; } var limit = 4, sum = 0; eval('sum = 5'); for (var index = 0; index < limit; index++) sum += addOne(index); sum + ':' + index;",
            "15:4",
        ),
        (
            "function addOne(value) { return value + 1; } var limit = 4, sum = 0; (0, eval)('sum = 5'); for (var index = 0; index < limit; index++) sum += addOne(index); sum + ':' + index;",
            "15:4",
        ),
        (
            "function addOne(value) { return value + 1; } var limit = 4, sum = 0; eval.call(undefined, 'sum = 5'); for (var index = 0; index < limit; index++) sum += addOne(index); sum + ':' + index;",
            "15:4",
        ),
        (
            "function addOne(value) { return value + 1; } var limit = 4, sum = 0; Reflect.apply(eval, undefined, ['sum = 5']); for (var index = 0; index < limit; index++) sum += addOne(index); sum + ':' + index;",
            "15:4",
        ),
        (
            "function addOne(value) { return value + 1; } var limit = 4, sum = 0; Function('sum = 5')(); for (var index = 0; index < limit; index++) sum += addOne(index); sum + ':' + index;",
            "15:4",
        ),
        (
            "function addOne(value) { return value + 1; } var limit = 4, sum = 0; with ({}) {} for (var index = 0; index < limit; index++) sum += addOne(index); sum + ':' + index;",
            "10:4",
        ),
    ] {
        assert_string(source, expected);
    }
    assert_string(
        "var limit = 4, reads = 0; \
         Object.defineProperty(globalThis, 'sloppySum', { configurable: true, \
           get: function () { reads += 1; return 1; } }); \
         for (var index = 0; index < limit; index++) sloppySum += index; \
         delete globalThis.sloppySum; sloppySum = 9; \
         reads + ':' + sloppySum;",
        "4:9",
    );
    for (limit, expected) in [("0", "0:0"), ("'3'", "6:3"), ("NaN", "0:0")] {
        assert_string(
            &format!(
                "var sum = 0; for (var index = 0; index < {limit}; index++) sum += index + 1; sum + ':' + index;"
            ),
            expected,
        );
    }
}

#[test]
fn eval_loops_never_publish_compiler_temporaries() {
    let scratch_count = "Object.getOwnPropertyNames(globalThis).filter(function (name) { \
        return name.length >= 2 && name.charCodeAt(0) === 0 && name.charCodeAt(1) === 0; \
    }).length";
    for eval_call in [
        "eval('var evalTotal = 0; for (var evalIndex = 0; evalIndex < 4; evalIndex++) evalTotal += evalIndex;')",
        "(0, eval)('var evalTotal = 0; for (var evalIndex = 0; evalIndex < 4; evalIndex++) evalTotal += evalIndex;')",
    ] {
        assert_string(
            &format!("{eval_call}; evalTotal + ':' + ({scratch_count});"),
            "6:0",
        );
    }
    assert_string(
        "function run() { \
            var total = 10; \
            eval('for (var inner = 0; inner < 4; inner++) total += inner;'); \
            for (var outer = 0; outer < 3; outer++) total += outer; \
            return total; \
        } \
        run() + ':' + Object.getOwnPropertyNames(globalThis).filter(function (name) { \
            return name.length >= 2 && name.charCodeAt(0) === 0 && name.charCodeAt(1) === 0; \
        }).length;",
        "19:0",
    );
}

#[test]
fn accumulation_loops_over_literals_declared_in_the_function() {
    assert_number(
        "function run(n) { var value = { a: 1, b: 2, c: 3 }; var total = 0; for (var i = 0; i < n; i++) { total += value.a; total += value.b; total += value.c; } return total; } run(5);",
        30.0,
    );
    assert_number(
        "function run(n) { var value = [1, 2, 3, 4]; var total = 0; for (var i = 0; i < n; i++) { total += value[0]; total += value[1]; total += value[2]; total += value[3]; } return total; } run(5);",
        50.0,
    );
    assert_number(
        "function run(n) { var add = function (value) { return value + 1; }; var total = 0; for (var i = 0; i < n; i++) { total += add(i); } return total; } run(5);",
        15.0,
    );
    assert_string(
        "function run(n) { var reads = 0, o = {}, key = 'a', sum = 0; Object.defineProperty(o, 'a', { get: function () { reads += 1; return 2; } }); for (var i = 0; i < n; i++) { sum += o[key]; } return sum + ':' + reads; } run(4);",
        "8:4",
    );
    assert_number(
        "var value = 2; function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += value; } return s; } sum(4);",
        8.0,
    );
}

#[test]
fn accumulation_loops_calling_numeric_leaves() {
    for (source, expected) in [
        (
            "function addOne(value) { return value + 1; } \
             function run(iterations) { var checksum = 0; \
               for (var i = 0; i < iterations; i++) checksum += addOne(i); \
               return checksum; } run(1000);",
            500500.0,
        ),
        (
            "function run(iterations) { \
               var receiver = { addOne: function (value) { return value + 1; } }; \
               var checksum = 0; \
               for (var i = 0; i < iterations; i++) checksum += receiver.addOne(i); \
               return checksum; } run(1000);",
            500500.0,
        ),
        (
            "var broadGlobalOne = 1; \
             function run(iterations) { var checksum = 0; \
               for (var i = 0; i < iterations; i++) checksum += broadGlobalOne; \
               return checksum; } run(1000);",
            1000.0,
        ),
        (
            "function leaf(value) { return value + 1; } function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s = leaf(i) + s; } return s; } sum(1000);",
            500500.0,
        ),
        (
            "function sum(n) { var offset = 1; var leaf = function (value) { return value + offset; }; var s = 0; for (var i = 0; i < n; i++) { s = leaf(i) + s; } return s; } sum(1000);",
            500500.0,
        ),
        (
            "function leaf(value) { if (value === 1) { leaf = function (next) { return next + 10; }; } return value + 1; } function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s = leaf(i) + s; } return s; } sum(3);",
            15.0,
        ),
    ] {
        assert_number(source, expected);
    }
    assert_string(
        "function leaf(value) { return 'x' + value; } function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s = leaf(i) + s; } return s; } sum(3);",
        "x2x1x00",
    );
}

#[test]
fn accumulation_loops_over_string_slice_lengths() {
    for (source, expected) in [
        (
            "function sum(n) { var text = 'the quick brown fox'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(1, 4).length; } return s; } sum(1000);",
            3000.0,
        ),
        (
            "function sum(n) { var text = '😀x'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(0, 1).length; } return s; } sum(4);",
            4.0,
        ),
        (
            "function sum(n) { var text = '😀x'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(i, 3).length; } return s; } sum(4);",
            6.0,
        ),
        (
            "function sum(n) { var text = '\\u{F0000}x'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(i, 3).length; } return s; } sum(4);",
            6.0,
        ),
        (
            "function sum(n) { var text = 'abcdef'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(-3, -1).length; } return s; } sum(4);",
            8.0,
        ),
        (
            "String.prototype.slice = function () { return { length: 7 }; }; function sum(n) { var text = 'abc'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(1, 2).length; } return s; } sum(4);",
            28.0,
        ),
    ] {
        assert_number(source, expected);
    }
    assert_string(
        "var reads = 0; var slice = String.prototype.slice; Object.defineProperty(String.prototype, 'slice', { get: function () { reads += 1; return slice; } }); function sum(n) { var text = 'abc'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(1, 2).length; } return s + ':' + reads; } sum(4);",
        "4:4",
    );
    // The index normalization the removed direct length helper implemented:
    // fractional, negative, reversed, non-finite and out-of-range bounds, and
    // code-unit (not code-point) positions.
    for (text, start, end, expected) in [
        ("'abcdef'", "1", "4", 3.0),
        ("'abcdef'", "1.9", "4.9", 3.0),
        ("'abcdef'", "-4", "-1", 3.0),
        ("'abcdef'", "4", "1", 0.0),
        ("'😀x'", "0", "1", 1.0),
        ("'😀x'", "0", "2", 2.0),
        ("'😀x'", "1", "2", 1.0),
        ("'\\uD800a\\uDC00'", "0", "1", 1.0),
        ("'\\uD800a\\uDC00'", "0", "3", 3.0),
        ("'abcdef'", "NaN", "Infinity", 6.0),
        ("'abcdef'", "-Infinity", "-1", 5.0),
        ("'abcdef'", "Infinity", "-Infinity", 0.0),
        ("'abcdef'", "-100", "100", 6.0),
    ] {
        assert_number(
            &format!(
                "function sum(n) {{ var text = {text}; var s = 0; for (var i = 0; i < n; i++) {{ s += text.slice({start}, {end}).length; }} return s; }} sum(3);"
            ),
            expected * 3.0,
        );
    }
}

// Loops whose body first selects one of two locals from a bitwise test of the
// counter and then accumulates a read or call through the selected value.

#[test]
fn selected_method_loops_preserve_zero_one_and_many_iterations() {
    let source = "function run(n) { var first = { f: function (value, offset) { return value + offset; } }; var second = { f: function (value, offset) { return value - offset; } }; var receiver; var sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 3) === 2 ? first : second; sum += receiver.f(i, 2); } return sum + ':' + (receiver === first ? 'first' : receiver === second ? 'second' : 'none'); }";
    for (count, expected) in [
        (0, "0:none"),
        (1, "-2:second"),
        (2, "-3:second"),
        (3, "1:first"),
        (6, "7:second"),
    ] {
        assert_string(&format!("{source} run({count});"), expected);
    }
}

#[test]
fn selected_receiver_loops_over_methods_reads_calls_and_builtins() {
    for (source, expected) in [
        (
            "function run(n) { var first = { f: function (value) { return value + 3; } }; var second = { f: function (value) { return value + 20; } }; var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 7) === 4 ? first : second; sum += receiver.f(i); } return sum; } run(9);",
            199.0,
        ),
        (
            "function run(n) { var source = { f: function (value) { return value + 1; } }; var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? source : source; sum += receiver.f(i); } return sum; } run(6);",
            21.0,
        ),
        (
            "function run(n) { var left = 3, right = 20; var first = { f: function (value) { return value + left; } }; var second = { f: function (value) { return value + right; } }; var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; } run(6);",
            84.0,
        ),
        (
            "function run(n) { var first = { value: 2 }, second = { value: 5 }, selected, sum = 0; for (var i = 0; i < n; i++) { selected = (i & 1) === 0 ? first : second; sum += selected.value; } return sum; } run(6);",
            21.0,
        ),
        (
            "function run(n) { var first = [2], second = [5], selected, sum = 0; for (var i = 0; i < n; i++) { selected = (i & 1) === 0 ? first : second; sum += selected[0]; } return sum; } run(6);",
            21.0,
        ),
        (
            "function run(n) { var first = { a: 2 }, second = { a: 5 }, key = 'a', selected, sum = 0; for (var i = 0; i < n; i++) { selected = (i & 1) === 0 ? first : second; sum += selected[key]; } return sum; } run(6);",
            21.0,
        ),
        (
            "function run(n) { var first = function (value) { return value + 1; }, second = function (value) { return value + 10; }, selected, sum = 0; for (var i = 0; i < n; i++) { selected = (i & 1) === 0 ? first : second; sum += selected(i); } return sum; } run(6);",
            48.0,
        ),
        (
            "function run(n) { var first = 'abcd', second = 'x', selected, sum = 0; for (var i = 0; i < n; i++) { selected = (i & 1) === 0 ? first : second; sum += selected.slice(1, 3).length; } return sum; } run(6);",
            6.0,
        ),
        (
            "function run(n) { var first = [1, 3], second = [3, 1], selected, sum = 0; for (var i = 0; i < n; i++) { selected = (i & 1) === 0 ? first : second; sum += selected.indexOf(3); } return sum; } run(6);",
            3.0,
        ),
    ] {
        assert_number(source, expected);
    }
}

#[test]
fn selected_receiver_loops_with_observable_or_non_numeric_methods() {
    assert_string(
        "function run(n) { var reads = 0, first = {}, second = {}; function f(value) { return value + 1; } Object.defineProperty(first, 'f', { get: function () { reads += 1; return f; } }); Object.defineProperty(second, 'f', { get: function () { reads += 1; return f; } }); var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum + ':' + reads; } run(4);",
        "10:4",
    );
    assert_number(
        "function run(n) { var proto = { f: function (value) { return value + 1; } }; var first = Object.create(proto), second = Object.create(proto), receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; } run(4);",
        10.0,
    );
    assert_number(
        "function run(n) { var first = new Proxy({ f: function (value) { return value + 1; } }, {}), second = new Proxy({ f: function (value) { return value + 1; } }, {}), receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; } run(4);",
        10.0,
    );
    assert_string(
        "function run(n) { var first = { f: function (value) { return value + 1; } }, second = { f: 1 }, receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; } try { run(4); 'missed'; } catch (error) { 'caught'; }",
        "caught",
    );
    assert_string(
        "function run(n) { var first = { f: function () { return 'a'; } }, second = { f: function () { return 'b'; } }, receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(); } return sum; } run(4);",
        "0abab",
    );
    assert_string(
        "function run(n) { var writes = 0; var first = { f: function () { writes += 1; return writes; } }, second = { f: function () { writes += 10; return writes; } }, receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(); } return sum + ':' + writes; } run(4);",
        "46:22",
    );
}

#[test]
fn selected_receiver_loops_whose_callees_observe_or_mutate_loop_state() {
    for (source, expected) in [
        (
            "function run(n) { var first = { f: function (value) { return value + i; } }, second = { f: function (value) { return value + i; } }, receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; } run(4);",
            12.0,
        ),
        (
            "function run(n) { var first = { f: function (value) { return value + sum; } }, second = { f: function (value) { return value + sum; } }, receiver, sum = 1; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; } run(4);",
            27.0,
        ),
        (
            "function run(n) { var first = {}, second = {}; first.f = function (value) { second.f = function (next) { return next + 100; }; return value + 1; }; second.f = function (value) { return value + 2; }; var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; } run(4);",
            208.0,
        ),
        (
            "function run(n) { var first, second, receiver, sum = 0; first = { f: function (value) { return value + (receiver === first ? 1 : 10); } }; second = { f: function (value) { return value + (receiver === first ? 1 : 10); } }; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; } run(4);",
            28.0,
        ),
        (
            "function run(n) { eval(''); var first = { f: function (value) { return value + 1; } }, second = { f: function (value) { return value + 1; } }, receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; } run(4);",
            10.0,
        ),
        (
            "function run(n) { with ({}) {} var first = { f: function (value) { return value + 1; } }, second = { f: function (value) { return value + 1; } }, receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; } run(4);",
            10.0,
        ),
    ] {
        assert_number(source, expected);
    }
}
