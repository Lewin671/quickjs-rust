use crate::{Value, eval};

#[test]
fn indexed_write_routes_through_per_kind_conversion() {
    // Direct `ta[i] = v` writes apply the per-kind numeric conversion and
    // persist through the backing buffer (IntegerIndexedElementSet).
    assert_eq!(
        eval(
            "let a = new Uint8Array(3); a[0] = 257; a[1] = -1; a[2] = 3.9; \
             a[0] + ',' + a[1] + ',' + a[2] + '|' \
             + Array.prototype.join.call(new Uint8Array(a.buffer));"
        ),
        Ok(Value::String("1,255,3|1,255,3".to_owned()))
    );
    assert_eq!(
        eval("let c = new Uint8ClampedArray(1); c[0] = 300; c[0];"),
        Ok(Value::Number(255.0))
    );
    assert_eq!(
        eval("let b = new BigInt64Array(1); b[0] = 5n; typeof b[0] + ':' + b[0];"),
        Ok(Value::String("bigint:5".to_owned()))
    );
    assert_eq!(
        eval("let a = new Uint8Array([1]); Reflect.set(a, 0, 257) + ':' + a[0];"),
        Ok(Value::String("true:1".to_owned()))
    );
}

#[test]
fn reflect_set_typed_array_index_uses_receiver_define_semantics() {
    assert_eq!(
        eval(
            "let target = new Uint8Array([0]); let receiver = {}; \
             let value = { valueOf() { throw 'coerced'; } }; \
             Reflect.set(target, 0, value, receiver) + ':' + target[0] + ':' + (receiver[0] === value);"
        ),
        Ok(Value::String("true:0:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let target = new Uint8Array([0]); let receiver = new Uint8Array([1]); \
             Reflect.set(target, 0, 257, receiver) + ':' + target[0] + ':' + receiver[0];"
        ),
        Ok(Value::String("true:0:1".to_owned()))
    );
    assert_eq!(
        eval(
            "let target = new Uint8Array([0, 0]); let receiver = new Uint8Array([1]); \
             let calls = 0; let value = { valueOf() { calls += 1; return 2; } }; \
             Reflect.set(target, 1, value, receiver) + ':' + target[1] + ':' \
             + receiver.hasOwnProperty(1) + ':' + calls;"
        ),
        Ok(Value::String("false:0:false:0".to_owned()))
    );
    assert_eq!(
        eval(
            "let target = new BigInt64Array([0n]); let receiver = new BigInt64Array([1n]); \
             Reflect.set(target, 0, Object(2n), receiver) + ':' + target[0] + ':' + receiver[0];"
        ),
        Ok(Value::String("true:0:2".to_owned()))
    );
}

#[test]
fn indexed_write_drops_out_of_range_and_canonical_indices() {
    // Out-of-range and non-integer canonical numeric indices never create a
    // property, but coercion side effects still run.
    assert_eq!(
        eval("let a = new Uint8Array(2); a[5] = 9; a[5] === undefined && a.length === 2;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let a = new Uint8Array(2); a['1.5'] = 7; a['1.5'];"),
        Ok(Value::Undefined)
    );
    // ToNumber side effects fire even for an out-of-range index.
    assert_eq!(
        eval(
            "let log = []; let a = new Uint8Array(1); \
             a[3] = { valueOf() { log.push('x'); return 0; } }; log.join(',');"
        ),
        Ok(Value::String("x".to_owned()))
    );
}

#[test]
fn non_numeric_property_writes_still_work() {
    assert_eq!(
        eval("let a = new Uint8Array(1); a.foo = 'bar'; a.foo;"),
        Ok(Value::String("bar".to_owned()))
    );
}

#[test]
fn detached_typed_array_numeric_delete_succeeds() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([1]); a.buffer.transfer(); \
             delete a[0] && delete a['-0'] && delete a['1.1'];"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let a = new BigInt64Array([1n]); a.buffer.transfer(); \
             Reflect.deleteProperty(a, 0) + ':' + Reflect.deleteProperty(a, '-0');"
        ),
        Ok(Value::String("true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new BigInt64Array([1n]); a.extra = 1; a.buffer.transfer(); \
             delete a.extra + ':' + a.hasOwnProperty('extra');"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
}
