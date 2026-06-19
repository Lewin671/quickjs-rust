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
        Ok(Value::String("1,255,3|1,255,3".to_owned().into()))
    );
    assert_eq!(
        eval("let c = new Uint8ClampedArray(1); c[0] = 300; c[0];"),
        Ok(Value::Number(255.0))
    );
    assert_eq!(
        eval("let b = new BigInt64Array(1); b[0] = 5n; typeof b[0] + ':' + b[0];"),
        Ok(Value::String("bigint:5".to_owned().into()))
    );
    assert_eq!(
        eval("let a = new Uint8Array([1]); Reflect.set(a, 0, 257) + ':' + a[0];"),
        Ok(Value::String("true:1".to_owned().into()))
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
        Ok(Value::String("true:0:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let target = new Uint8Array([0]); let receiver = new Uint8Array([1]); \
             Reflect.set(target, 0, 257, receiver) + ':' + target[0] + ':' + receiver[0];"
        ),
        Ok(Value::String("true:0:1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let target = new Uint8Array([0, 0]); let receiver = new Uint8Array([1]); \
             let calls = 0; let value = { valueOf() { calls += 1; return 2; } }; \
             Reflect.set(target, 1, value, receiver) + ':' + target[1] + ':' \
             + receiver.hasOwnProperty(1) + ':' + calls;"
        ),
        Ok(Value::String("false:0:false:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let target = new BigInt64Array([0n]); let receiver = new BigInt64Array([1n]); \
             Reflect.set(target, 0, Object(2n), receiver) + ':' + target[0] + ':' + receiver[0];"
        ),
        Ok(Value::String("true:0:2".to_owned().into()))
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
        Ok(Value::String("x".to_owned().into()))
    );
}

#[test]
fn non_numeric_property_writes_still_work() {
    assert_eq!(
        eval("let a = new Uint8Array(1); a.foo = 'bar'; a.foo;"),
        Ok(Value::String("bar".to_owned().into()))
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
        Ok(Value::String("true:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = new BigInt64Array([1n]); a.extra = 1; a.buffer.transfer(); \
             delete a.extra + ':' + a.hasOwnProperty('extra');"
        ),
        Ok(Value::String("true:false".to_owned().into()))
    );
}

#[test]
fn own_property_descriptor_uses_current_indexed_element_state() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(2, { maxByteLength: 4 }); \
             let a = new Uint8Array(b); a[1] = 7; \
             let before = Object.getOwnPropertyDescriptor(a, 1); \
             b.resize(3); a[2] = 9; \
             let grown = Object.getOwnPropertyDescriptor(a, 2); \
             b.transfer(); \
             let detached = Object.getOwnPropertyDescriptor(a, 1); \
             before.value + ':' + before.enumerable + ':' + before.writable + ':' \
             + before.configurable + '|' + grown.value + '|' + (detached === undefined);"
        ),
        Ok(Value::String("7:true:true:true|9|true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = new BigInt64Array([1n]); a.buffer.transfer(); \
             Object.getOwnPropertyDescriptor(a, 0) === undefined;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn indexed_reads_use_current_resizable_buffer_bounds() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let fixed = new Uint8Array(b, 0, 4); \
             let tracking = new Uint8Array(b, 1); \
             fixed.set([10, 20, 30, 40]); \
             b.resize(2); \
             let shrunk = (fixed[0] === undefined) + ':' + (fixed[2] === undefined) + ':' \
               + tracking.length + ':' + tracking[0] + ':' + (tracking[1] === undefined); \
             b.resize(5); \
             shrunk + '|' + fixed.length + ':' + fixed[2] + ':' \
               + tracking.length + ':' + tracking[3];"
        ),
        Ok(Value::String(
            "true:true:1:20:true|4:0:4:0".to_owned().into()
        ))
    );
}

#[test]
fn define_own_property_uses_integer_indexed_semantics() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([1, 2]); \
             let ok = Reflect.defineProperty(a, '0', { \
               value: 257, configurable: true, enumerable: true, writable: true \
             }); \
             let rejectWritable = Reflect.defineProperty(a, '1', { \
               value: 9, configurable: true, enumerable: true, writable: false \
             }); \
             let rejectOutOfRange = Reflect.defineProperty(a, '2', { \
               value: 9, configurable: true, enumerable: true, writable: true \
             }); \
             let rejectFractional = Reflect.defineProperty(a, '0.5', { \
               value: 9, configurable: true, enumerable: true, writable: true \
             }); \
             ok + ':' + a[0] + ':' + rejectWritable + ':' + a[1] + ':' \
             + rejectOutOfRange + ':' + (a[2] === undefined) + ':' \
             + rejectFractional + ':' + (a['0.5'] === undefined);"
        ),
        Ok(Value::String(
            "true:1:false:2:false:true:false:true".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let a = new BigInt64Array([1n]); \
             Reflect.defineProperty(a, '0', { value: Object(2n) }) + ':' + a[0];"
        ),
        Ok(Value::String("true:2".to_owned().into()))
    );
}

#[test]
fn object_define_property_throws_for_rejected_typed_array_indices() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([1]); let descriptor = Object.getOwnPropertyDescriptor(a, '0'); \
             let badIndex = false; let badAttrs = false; \
             try { Object.defineProperty(a, '-0', descriptor); } \
             catch (e) { badIndex = e instanceof TypeError; } \
             try { Object.defineProperty(a, '0', { configurable: false }); } \
             catch (e) { badAttrs = e instanceof TypeError; } \
             badIndex + ':' + badAttrs + ':' + a[0];"
        ),
        Ok(Value::String("true:true:1".to_owned().into()))
    );
}

#[test]
fn typed_array_define_property_coercion_order_matches_integer_indexed_set() {
    assert_eq!(
        eval(
            "let calls = 0; let a = new Uint8Array([1]); \
             let value = { valueOf() { calls += 1; return 258; } }; \
             let ok = Reflect.defineProperty(a, '0', { value }); \
             let rejected = Reflect.defineProperty(a, '1', { value }); \
             ok + ':' + rejected + ':' + calls + ':' + a[0] + ':' + (a[1] === undefined);"
        ),
        Ok(Value::String("true:false:1:2:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = new BigInt64Array([17n]); \
             let ok = Reflect.defineProperty(a, '0', { \
               value: { valueOf() { a.buffer.transfer(); return 42n; } } \
             }); \
             ok + ':' + (a[0] === undefined);"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn detached_typed_array_for_in_skips_stale_index_descriptors() {
    assert_eq!(
        eval(
            "let a = new Uint8Array(4); let before = 0; \
             for (let key in a) { before += 1; } \
             a.buffer.transfer(); let after = 0; \
             for (let key in a) { after += 1; } \
             before + ':' + after;"
        ),
        Ok(Value::String("4:0".to_owned().into()))
    );
}

#[test]
fn own_keys_track_resizable_typed_array_length() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 6 }); \
             let a = new Uint8Array(b, 1); \
             Reflect.ownKeys(a).join(',') + '|'; \
             b.resize(6); let grown = Reflect.ownKeys(a).join(','); \
             b.resize(3); let shrunk = Reflect.ownKeys(a).join(','); \
             b.resize(1); let boundary = Reflect.ownKeys(a).join(','); \
             grown + '|' + shrunk + '|' + boundary;"
        ),
        Ok(Value::String("0,1,2,3,4|0,1|".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 6 }); \
             let a = new Uint8Array(b, 1, 2); \
             let initial = Reflect.ownKeys(a).join(','); \
             b.resize(6); let grown = Reflect.ownKeys(a).join(','); \
             b.resize(2); let out = Reflect.ownKeys(a).join(','); \
             initial + '|' + grown + '|' + out;"
        ),
        Ok(Value::String("0,1|0,1|".to_owned().into()))
    );
}

#[test]
fn own_keys_keep_non_index_properties_after_dynamic_indices() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 6 }); \
             let a = new Uint8Array(b); a.extra = 1; \
             b.resize(2); \
             Object.keys(a).join(',') + '|' + Object.getOwnPropertyNames(a).join(',');"
        ),
        Ok(Value::String("0,1,extra|0,1,extra".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(2, { maxByteLength: 4 }); \
             let a = new Uint8Array(b); let s = Symbol('s'); a[s] = 1; \
             b.resize(3); let keys = Reflect.ownKeys(a); \
             keys.length + ':' + keys.slice(0, 3).join(',') + ':' + (keys[3] === s);"
        ),
        Ok(Value::String("4:0,1,2:true".to_owned().into()))
    );
}
