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
fn data_view_constructor_length_and_name() {
    assert_eq!(eval("DataView.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("DataView.name;"),
        Ok(Value::String("DataView".to_owned()))
    );
}

#[test]
fn data_view_to_string_tag_is_data_property() {
    assert_eq!(
        eval("Object.prototype.toString.call(new DataView(new ArrayBuffer(4)));"),
        Ok(Value::String("[object DataView]".to_owned()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(DataView.prototype, Symbol.toStringTag); \
             [d.value, d.writable, d.enumerable, d.configurable].join(',');"
        ),
        Ok(Value::String("DataView,false,false,true".to_owned()))
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
fn data_view_bigint_round_trips() {
    assert_eq!(
        eval("let v = new DataView(new ArrayBuffer(8)); v.setBigInt64(0, -1n); v.getBigInt64(0);"),
        Ok(Value::BigInt(num_bigint::BigInt::from(-1)))
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
        Ok(Value::String("value".to_owned()))
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
        Ok(Value::String("offset,value".to_owned()))
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
