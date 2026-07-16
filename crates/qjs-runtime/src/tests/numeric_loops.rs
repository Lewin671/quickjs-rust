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
