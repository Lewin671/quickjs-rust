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
