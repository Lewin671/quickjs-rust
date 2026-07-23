use crate::{Value, eval};

// --- constructor -------------------------------------------------------------

#[test]
fn data_view_constructor_defaults_to_buffer_remainder() {
    assert_eq!(
        eval("new DataView(new ArrayBuffer(8)).byteLength;"),
        Ok(Value::Number(8.0))
    );
    assert_eq!(
        eval("new DataView(new ArrayBuffer(8), 2).byteLength;"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("new DataView(new ArrayBuffer(8), 2, 3).byteLength;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("new DataView(new ArrayBuffer(8), 2).byteOffset;"),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn data_view_buffer_accessor_returns_backing_buffer() {
    assert_eq!(
        eval("let b = new ArrayBuffer(8); new DataView(b).buffer === b;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let b = new SharedArrayBuffer(8); new DataView(b).buffer === b;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn data_view_accepts_shared_array_buffer() {
    assert_eq!(
        eval(
            "let b = new SharedArrayBuffer(8); \
             let v = new DataView(b, 2, 4); \
             [v.byteLength, v.byteOffset, v.buffer === b].join(':');"
        ),
        Ok(Value::String("4:2:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let b = new SharedArrayBuffer(8); \
             let v = new DataView(b); \
             v.setUint16(0, 0x1234); \
             v.getUint8(0) * 256 + v.getUint8(1);"
        ),
        Ok(Value::Number(4660.0))
    );
}

#[test]
fn data_view_tracks_growable_shared_buffer_only_without_explicit_length() {
    assert_eq!(
        eval(
            "let buffer = new SharedArrayBuffer(2, { maxByteLength: 8 }); \
             let tracking = new DataView(buffer); \
             let fixed = new DataView(buffer, 0, 2); \
             buffer.grow(6); \
             [tracking.byteLength, fixed.byteLength].join(':');"
        ),
        Ok(Value::String("6:2".to_owned().into()))
    );
}

#[test]
fn data_view_requires_array_buffer() {
    assert!(eval("new DataView({});").is_err());
    assert!(eval("new DataView(new Uint8Array(8));").is_err());
    assert!(eval("new DataView();").is_err());
}

#[test]
fn data_view_not_callable_without_new() {
    assert!(eval("DataView(new ArrayBuffer(8));").is_err());
}

#[test]
fn data_view_rejects_out_of_bounds_offset_and_length() {
    assert!(eval("new DataView(new ArrayBuffer(8), 9);").is_err());
    assert!(eval("new DataView(new ArrayBuffer(8), 4, 5);").is_err());
    assert!(eval("new DataView(new ArrayBuffer(8), -1);").is_err());
}

#[test]
fn data_view_constructor_reads_custom_new_target_prototype_after_offset() {
    assert_eq!(
        eval(
            "let buffer = new ArrayBuffer(8); \
             let calls = []; \
             let offset = { valueOf() { calls.push('offset'); return 1; } }; \
             let newTarget = (function() {}).bind(null); \
             Object.defineProperty(newTarget, 'prototype', { \
               get() { calls.push('prototype'); return DataView.prototype; } \
             }); \
             Reflect.construct(DataView, [buffer, offset], newTarget); \
             calls.join(':');"
        ),
        Ok(Value::String("offset:prototype".to_owned().into()))
    );
}

#[test]
fn data_view_constructor_propagates_custom_new_target_prototype_throw() {
    assert_eq!(
        eval(
            "let buffer = new ArrayBuffer(8); \
             let marker = {}; \
             let newTarget = (function() {}).bind(null); \
             Object.defineProperty(newTarget, 'prototype', { get() { throw marker; } }); \
             let caught = false; \
             try { Reflect.construct(DataView, [buffer, 0], newTarget); } catch (e) { caught = e === marker; } \
             caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn data_view_constructor_rechecks_detached_buffer_after_custom_prototype_access() {
    assert_eq!(
        eval(
            "let buffer = new ArrayBuffer(8); \
             let newTarget = (function() {}).bind(null); \
             Object.defineProperty(newTarget, 'prototype', { \
               get() { __quickjsRustDetachArrayBuffer(buffer); return DataView.prototype; } \
             }); \
             let threw = false; \
             try { Reflect.construct(DataView, [buffer, 0], newTarget); } catch (e) { threw = e instanceof TypeError; } \
             threw;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn data_view_constructor_rechecks_resized_buffer_after_custom_prototype_access() {
    assert_eq!(
        eval(
            "let validBuffer = new ArrayBuffer(3, { maxByteLength: 3 }); \
             let validTarget = (function() {}).bind(null); \
             Object.defineProperty(validTarget, 'prototype', { \
               get() { validBuffer.resize(2); return DataView.prototype; } \
             }); \
             let valid = Reflect.construct(DataView, [validBuffer, 2], validTarget); \
             let invalidBuffer = new ArrayBuffer(3, { maxByteLength: 3 }); \
             let invalidTarget = (function() {}).bind(null); \
             Object.defineProperty(invalidTarget, 'prototype', { \
               get() { invalidBuffer.resize(2); return DataView.prototype; } \
             }); \
             let invalidThrows = false; \
             try { Reflect.construct(DataView, [invalidBuffer, 1, 2], invalidTarget); } \
             catch (e) { invalidThrows = e instanceof RangeError; } \
             [valid.byteLength, invalidThrows].join(':');"
        ),
        Ok(Value::String("0:true".to_owned().into()))
    );
}

#[test]
fn data_view_constructor_length_and_name() {
    assert_eq!(eval("DataView.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("DataView.name;"),
        Ok(Value::String("DataView".to_owned().into()))
    );
}

#[test]
fn data_view_to_string_tag_is_data_property() {
    assert_eq!(
        eval("Object.prototype.toString.call(new DataView(new ArrayBuffer(4)));"),
        Ok(Value::String("[object DataView]".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(DataView.prototype, Symbol.toStringTag); \
             [d.value, d.writable, d.enumerable, d.configurable].join(',');"
        ),
        Ok(Value::String("DataView,false,false,true".to_owned().into()))
    );
}

// --- endianness round-trips --------------------------------------------------

#[test]
fn data_view_uint16_big_and_little_endian() {
    assert_eq!(
        eval("let v = new DataView(new ArrayBuffer(8)); v.setUint16(0, 0x1234); v.getUint16(0);"),
        Ok(Value::Number(4660.0))
    );
    // Big-endian stores the high byte first.
    assert_eq!(
        eval(
            "let v = new DataView(new ArrayBuffer(8)); v.setUint16(0, 0x1234); \
             v.getUint8(0) * 256 + v.getUint8(1);"
        ),
        Ok(Value::Number(4660.0))
    );
    // Little-endian byte order is reversed.
    assert_eq!(
        eval(
            "let v = new DataView(new ArrayBuffer(8)); v.setUint16(0, 0x1234, true); \
             v.getUint8(0);"
        ),
        Ok(Value::Number(0x34 as f64))
    );
    assert_eq!(
        eval(
            "let v = new DataView(new ArrayBuffer(8)); v.setUint16(0, 0x1234, true); \
             v.getUint16(0, true);"
        ),
        Ok(Value::Number(4660.0))
    );
}

#[test]
fn data_view_int8_sign_extension() {
    assert_eq!(
        eval("let v = new DataView(new ArrayBuffer(4)); v.setInt8(0, -1); v.getInt8(0);"),
        Ok(Value::Number(-1.0))
    );
    assert_eq!(
        eval("let v = new DataView(new ArrayBuffer(4)); v.setInt8(0, -1); v.getUint8(0);"),
        Ok(Value::Number(255.0))
    );
}

#[test]
fn data_view_int32_round_trip_both_endians() {
    assert_eq!(
        eval(
            "let v = new DataView(new ArrayBuffer(8)); v.setInt32(0, -123456789); \
             v.getInt32(0);"
        ),
        Ok(Value::Number(-123456789.0))
    );
    assert_eq!(
        eval(
            "let v = new DataView(new ArrayBuffer(8)); v.setUint32(0, 0xDEADBEEF, true); \
             v.getUint32(0, true);"
        ),
        Ok(Value::Number(0xDEADBEEF_u32 as f64))
    );
    // Cross-endian read sees the byte-swapped value.
    assert_eq!(
        eval(
            "let v = new DataView(new ArrayBuffer(8)); v.setUint32(0, 0x01020304); \
             v.getUint32(0, true);"
        ),
        Ok(Value::Number(0x04030201_u32 as f64))
    );
}

#[test]
fn data_view_float_round_trips() {
    assert_eq!(
        eval(
            "let v = new DataView(new ArrayBuffer(8)); v.setFloat64(0, 123.456); \
             v.getFloat64(0);"
        ),
        Ok(Value::Number(123.456))
    );
    assert_eq!(
        eval(
            "let v = new DataView(new ArrayBuffer(8)); v.setFloat32(0, 1.5, true); \
             v.getFloat32(0, true);"
        ),
        Ok(Value::Number(1.5))
    );
    // Float32 stored value loses precision relative to the f64 input.
    assert_eq!(
        eval(
            "let v = new DataView(new ArrayBuffer(8)); v.setFloat32(0, 0.1); \
             Math.abs(v.getFloat32(0) - 0.1) < 1e-7;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn data_view_float16_round_trips_and_uses_expected_bytes() {
    assert_eq!(
        eval(
            "let v = new DataView(new ArrayBuffer(8)); \
             v.setFloat16(0, 1.5); \
             v.setFloat16(2, 42, true); \
             [v.getFloat16(0), v.getUint8(0), v.getUint8(1), \
              v.getFloat16(2, true), v.getUint8(2), v.getUint8(3)].join(':');"
        ),
        Ok(Value::String("1.5:62:0:42:64:81".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let v = new DataView(new ArrayBuffer(8)); \
             v.setFloat16(0, 0.333251953125); \
             v.getFloat16(0);"
        ),
        Ok(Value::Number(0.333251953125))
    );
}

#[test]
fn data_view_float16_handles_special_values() {
    assert_eq!(
        eval(
            "let v = new DataView(new ArrayBuffer(12)); \
             v.setUint8(0, 0x7c); v.setUint8(1, 0x00); \
             v.setUint8(2, 0xfc); v.setUint8(3, 0x00); \
             v.setUint8(4, 0x80); v.setUint8(5, 0x00); \
             v.setUint8(6, 0x00); v.setUint8(7, 0x01); \
             v.setFloat16(8, NaN); \
             [v.getFloat16(0), v.getFloat16(2), Object.is(v.getFloat16(4), -0), \
              v.getFloat16(6), Number.isNaN(v.getFloat16(8))].join(':');"
        ),
        Ok(Value::String(
            "Infinity:-Infinity:true:5.960464477539063e-8:true"
                .to_owned()
                .into()
        ))
    );
}

#[test]
fn data_view_float16_accessors_follow_resizable_buffer() {
    assert_eq!(
        eval(
            "let buffer = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let view = new DataView(buffer); \
             let values = [view.setFloat16(0, 10), view.getFloat16(0)]; \
             buffer.resize(1); \
             let getThrows = false; \
             let setThrows = false; \
             try { view.getFloat16(0); } catch (e) { getThrows = e instanceof RangeError; } \
             try { view.setFloat16(0, 20); } catch (e) { setThrows = e instanceof RangeError; } \
             values.push(getThrows, setThrows); \
             values.join(':');"
        ),
        Ok(Value::String(":10:true:true".to_owned().into()))
    );
}

#[test]
fn data_view_bigint_round_trips() {
    assert_eq!(
        eval("let v = new DataView(new ArrayBuffer(8)); v.setBigInt64(0, -1n); v.getBigInt64(0);"),
        Ok(Value::bigint(num_bigint::BigInt::from(-1)))
    );
    assert_eq!(
        eval(
            "let v = new DataView(new ArrayBuffer(8)); v.setBigUint64(0, -1n); \
             v.getBigUint64(0) === 18446744073709551615n;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let v = new DataView(new ArrayBuffer(8)); v.setBigInt64(0, 0x0102030405060708n, true); \
             v.getBigUint64(0) === 0x0807060504030201n;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn data_view_set_bigint_rejects_number() {
    assert!(eval("let v = new DataView(new ArrayBuffer(8)); v.setBigInt64(0, 1);").is_err());
}

// --- bounds / offset errors --------------------------------------------------

#[test]
fn data_view_get_out_of_bounds_throws_range_error() {
    assert!(eval("new DataView(new ArrayBuffer(4)).getInt32(1);").is_err());
    assert!(eval("new DataView(new ArrayBuffer(4)).getInt8(4);").is_err());
    assert!(eval("new DataView(new ArrayBuffer(8), 4).getInt32(1);").is_err());
}

#[test]
fn data_view_set_out_of_bounds_throws_range_error() {
    assert!(eval("new DataView(new ArrayBuffer(4)).setInt32(1, 0);").is_err());
}

#[test]
fn data_view_set_rejects_immutable_buffer_before_argument_coercion() {
    assert_eq!(
        eval(
            "let buffer = new ArrayBuffer(8).transferToImmutable(); \
             let view = new DataView(buffer); \
             let calls = []; \
             let offset = { valueOf() { calls.push('offset'); return 0; } }; \
             let value = { valueOf() { calls.push('value'); return 1; } }; \
             try { view.setUint8(offset, value); } catch (e) { calls.push(e instanceof TypeError); } \
             calls.join(':');"
        ),
        Ok(Value::String("true".to_owned().into()))
    );
}

#[test]
fn data_view_methods_reject_resized_out_of_bounds_view() {
    assert_eq!(
        eval(
            "let buffer = new ArrayBuffer(24, { maxByteLength: 32 }); \
             let view = new DataView(buffer, 0, 16); \
             buffer.resize(8); \
             let setThrows = false; \
             let getThrows = false; \
             try { view.setUint8(0, 1); } catch (e) { setThrows = e instanceof TypeError; } \
             try { view.getUint8(0); } catch (e) { getThrows = e instanceof TypeError; } \
             setThrows + ':' + getThrows;"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn data_view_length_tracking_accessors_follow_resizable_buffer() {
    assert_eq!(
        eval(
            "let buffer = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let view = new DataView(buffer, 1); \
             let values = [view.byteLength, view.byteOffset]; \
             buffer.resize(6); \
             values.push(view.byteLength, view.byteOffset); \
             buffer.resize(1); \
             values.push(view.byteLength, view.byteOffset); \
             values.join(':');"
        ),
        Ok(Value::String("3:1:5:1:0:1".to_owned().into()))
    );
}

#[test]
fn data_view_accessors_reject_resized_out_of_bounds_view() {
    assert_eq!(
        eval(
            "let fixedBuffer = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let fixed = new DataView(fixedBuffer, 1, 2); \
             fixedBuffer.resize(2); \
             let fixedLengthThrows = false; \
             let fixedOffsetThrows = false; \
             try { fixed.byteLength; } catch (e) { fixedLengthThrows = e instanceof TypeError; } \
             try { fixed.byteOffset; } catch (e) { fixedOffsetThrows = e instanceof TypeError; } \
             let trackingBuffer = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let tracking = new DataView(trackingBuffer, 1); \
             trackingBuffer.resize(0); \
             let trackingLengthThrows = false; \
             let trackingOffsetThrows = false; \
             try { tracking.byteLength; } catch (e) { trackingLengthThrows = e instanceof TypeError; } \
             try { tracking.byteOffset; } catch (e) { trackingOffsetThrows = e instanceof TypeError; } \
             [fixedLengthThrows, fixedOffsetThrows, trackingLengthThrows, trackingOffsetThrows].join(':');"
        ),
        Ok(Value::String("true:true:true:true".to_owned().into()))
    );
}

#[test]
fn data_view_negative_index_throws() {
    assert!(eval("new DataView(new ArrayBuffer(8)).getInt8(-1);").is_err());
}

// --- brand checks ------------------------------------------------------------

#[test]
fn data_view_accessors_brand_check() {
    assert!(
        eval("Object.getOwnPropertyDescriptor(DataView.prototype, 'byteLength').get.call({});")
            .is_err()
    );
    assert!(eval("DataView.prototype.getInt8.call({}, 0);").is_err());
    assert!(eval("DataView.prototype.setInt8.call({}, 0, 0);").is_err());
}

// --- detached behavior -------------------------------------------------------

#[test]
fn data_view_zero_length_view() {
    // No JS-facing buffer-detach API exists yet (tracked outside this slice), so
    // the detached-buffer guards are exercised by Test262's harness rather than
    // here. A zero-length view still resolves cleanly.
    assert_eq!(
        eval("new DataView(new ArrayBuffer(0)).byteLength;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("new DataView(new ArrayBuffer(0)).byteOffset;"),
        Ok(Value::Number(0.0))
    );
}

// --- coercion ordering -------------------------------------------------------

#[test]
fn data_view_set_coerces_value_before_bounds_check() {
    // The value's valueOf runs even though the offset is out of bounds: a
    // RangeError is still thrown, but only after the side effect fires.
    assert_eq!(
        eval(
            "let log = []; \
             let v = new DataView(new ArrayBuffer(4)); \
             let value = { valueOf() { log.push('value'); return 1; } }; \
             try { v.setInt32(8, value); } catch (e) {} \
             log.join(',');"
        ),
        Ok(Value::String("value".to_owned().into()))
    );
}

#[test]
fn data_view_set_coerces_index_before_value() {
    // ToIndex(offset) runs before ToNumber(value) per SetViewValue step order.
    assert_eq!(
        eval(
            "let log = []; \
             let v = new DataView(new ArrayBuffer(8)); \
             let offset = { valueOf() { log.push('offset'); return 0; } }; \
             let value = { valueOf() { log.push('value'); return 1; } }; \
             v.setInt32(offset, value); \
             log.join(',');"
        ),
        Ok(Value::String("offset,value".to_owned().into()))
    );
}

#[test]
fn data_view_get_coerces_index() {
    assert_eq!(
        eval(
            "let v = new DataView(new ArrayBuffer(8)); v.setUint8(2, 42); \
             v.getUint8({ valueOf() { return 2; } });"
        ),
        Ok(Value::Number(42.0))
    );
}

#[test]
fn data_view_subclassing_uses_new_target_prototype() {
    assert_eq!(
        eval(
            "class MyView extends DataView {} \
             let v = new MyView(new ArrayBuffer(8)); \
             v instanceof MyView && v instanceof DataView && v.byteLength === 8;"
        ),
        Ok(Value::Boolean(true))
    );
}
