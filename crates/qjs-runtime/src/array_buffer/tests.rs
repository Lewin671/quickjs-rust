use crate::{Value, eval};

#[test]
fn array_buffer_constructor_and_byte_length() {
    assert_eq!(
        eval("let buffer = new ArrayBuffer(8); buffer.byteLength;"),
        Ok(Value::Number(8.0))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new ArrayBuffer(0));"),
        Ok(Value::String("[object ArrayBuffer]".to_owned()))
    );
}

#[test]
fn array_buffer_slice_resolves_bounds() {
    assert_eq!(
        eval("new ArrayBuffer(8).slice(2, 6).byteLength;"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("new ArrayBuffer(8).slice(-5, -1).byteLength;"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("new ArrayBuffer(8).slice(9, 1).byteLength;"),
        Ok(Value::Number(0.0))
    );
}

#[test]
fn array_buffer_slice_rejects_non_object_constructor() {
    assert!(
        eval("let buffer = new ArrayBuffer(8); buffer.constructor = true; buffer.slice();")
            .is_err()
    );
}

#[test]
fn array_buffer_slice_validates_species_result_size() {
    assert_eq!(
        eval(
            "let species = {}; \
             species[Symbol.species] = function() { return new ArrayBuffer(10); }; \
             let buffer = new ArrayBuffer(8); \
             buffer.constructor = species; \
             buffer.slice().byteLength;"
        ),
        Ok(Value::Number(10.0))
    );
    assert!(
        eval(
            "let species = {}; \
             let buffer = new ArrayBuffer(8); \
             species[Symbol.species] = function() { return buffer; }; \
             buffer.constructor = species; \
             buffer.slice();"
        )
        .is_err()
    );
    assert!(
        eval(
            "let species = {}; \
             species[Symbol.species] = function() { return new ArrayBuffer(4); }; \
             let buffer = new ArrayBuffer(8); \
             buffer.constructor = species; \
             buffer.slice();"
        )
        .is_err()
    );
}

#[test]
fn array_buffer_slice_to_immutable_copies_and_marks_result() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(8); \
             let view = new Uint8Array(b); \
             view[0] = 1; view[1] = 2; view[2] = 3; view[3] = 4; \
             let c = b.sliceToImmutable(1, 4); \
             [c.byteLength, c.immutable, c.resizable, new Uint8Array(c)[0], new Uint8Array(c)[1], new Uint8Array(c)[2]].join(':');"
        ),
        Ok(Value::String("3:true:false:2:3:4".to_owned()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(8); \
             b.constructor = { [Symbol.species]: function() { return new ArrayBuffer(32); } }; \
             b.sliceToImmutable().byteLength;"
        ),
        Ok(Value::Number(8.0))
    );
}

#[test]
fn array_buffer_slice_to_immutable_validates_receivers_before_arguments() {
    assert_eq!(
        eval(
            "let calls = 0; \
             let start = { valueOf() { calls++; return 0; } }; \
             try { ArrayBuffer.prototype.sliceToImmutable.call({}, start); } catch (_) {} \
             calls;"
        ),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(8); \
             __quickjsRustDetachArrayBuffer(b); \
             let calls = 0; \
             let start = { valueOf() { calls++; return 0; } }; \
             try { b.sliceToImmutable(start); } catch (_) {} \
             calls;"
        ),
        Ok(Value::Number(0.0))
    );
}

#[test]
fn array_buffer_slice_to_immutable_rechecks_after_bounds_side_effects() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(10, { maxByteLength: 12 }); \
             let view = new Uint8Array(b); \
             for (let i = 0; i < 10; i++) view[i] = i + 1; \
             let start = { valueOf() { b.resize(11); return -7; } }; \
             let end = { valueOf() { b.resize(12); return -4; } }; \
             let c = b.sliceToImmutable(start, end); \
             [b.byteLength, c.byteLength, new Uint8Array(c)[0], new Uint8Array(c)[1], new Uint8Array(c)[2]].join(':');"
        ),
        Ok(Value::String("12:3:4:5:6".to_owned()))
    );
    assert!(
        eval(
            "let b = new ArrayBuffer(10, { maxByteLength: 10 }); \
             let start = { valueOf() { b.resize(9); return -7; } }; \
             let end = { valueOf() { b.resize(5); return -4; } }; \
             b.sliceToImmutable(start, end);"
        )
        .is_err()
    );
    assert!(
        eval(
            "let b = new ArrayBuffer(8); \
             let end = { valueOf() { __quickjsRustDetachArrayBuffer(b); return 1; } }; \
             b.sliceToImmutable(0, end);"
        )
        .is_err()
    );
}

#[test]
fn array_buffer_is_view_reports_typed_arrays() {
    assert_eq!(
        eval("ArrayBuffer.isView(new Uint8Array(4));"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("ArrayBuffer.isView(new DataView(new ArrayBuffer(4)));"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("ArrayBuffer.isView(new ArrayBuffer(4));"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("ArrayBuffer.isView({});"), Ok(Value::Boolean(false)));
    assert_eq!(eval("ArrayBuffer.isView();"), Ok(Value::Boolean(false)));
}

#[test]
fn array_buffer_species_is_self() {
    assert_eq!(
        eval("ArrayBuffer[Symbol.species] === ArrayBuffer;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn array_buffer_resize_and_resizable_accessors() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(8, { maxByteLength: 16 }); \
             b.resizable + ':' + b.byteLength + ':' + b.maxByteLength;"
        ),
        Ok(Value::String("true:8:16".to_owned()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(8, { maxByteLength: 16 }); \
             b.resize(12); b.byteLength + ':' + new Uint8Array(b)[10];"
        ),
        Ok(Value::String("12:0".to_owned()))
    );
    assert!(eval("new ArrayBuffer(8, { maxByteLength: 4 });").is_err());
    assert!(eval("let b = new ArrayBuffer(8); b.resize(4);").is_err());
    // A plain or undefined options argument must still succeed.
    assert_eq!(
        eval("let b = new ArrayBuffer(8, {}); b.resizable + ':' + b.maxByteLength;"),
        Ok(Value::String("false:8".to_owned()))
    );
    assert_eq!(
        eval("new ArrayBuffer(8, undefined).byteLength;"),
        Ok(Value::Number(8.0))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(8, { maxByteLength: 16 }); \
             __quickjsRustDetachArrayBuffer(b); b.resizable;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn array_buffer_resize_coerces_length_before_detached_check() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(8, { maxByteLength: 16 }); \
             let called = false; \
             try { b.resize({ valueOf() { called = true; __quickjsRustDetachArrayBuffer(b); return 4; } }); } catch (_) {} \
             called;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(8, { maxByteLength: 16 }); \
             __quickjsRustDetachArrayBuffer(b); \
             let called = false; \
             try { b.resize({ valueOf() { called = true; return 4; } }); } catch (_) {} \
             called;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn array_buffer_immutable_accessor_and_transfer() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4); \
             let a = new Uint8Array(b); a[0] = 7; a[1] = 8; \
             let c = b.transferToImmutable(6); \
             [b.byteLength, b.detached, c.byteLength, c.immutable, new Uint8Array(c)[0], new Uint8Array(c)[1], new Uint8Array(c)[5]].join(':');"
        ),
        Ok(Value::String("0:true:6:true:7:8:0".to_owned()))
    );
    assert_eq!(
        eval("let b = new ArrayBuffer(4); b.immutable;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let b = new ArrayBuffer(4); b.transferToImmutable(2).byteLength;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "new ArrayBuffer(4).transferToImmutable('\\t\\u000b\\u000c\\uFEFF\\u3000\\n\\r\\u2028\\u20291\\t\\u000b\\u000c\\uFEFF\\u3000\\n\\r\\u2028\\u2029').byteLength;"
        ),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn array_buffer_immutable_rejects_invalid_receivers_and_retransfer() {
    assert!(
        eval("Object.getOwnPropertyDescriptor(ArrayBuffer.prototype, 'immutable').get.call({});")
            .is_err()
    );
    assert!(
        eval("ArrayBuffer.prototype.transferToImmutable.call(new SharedArrayBuffer(4));").is_err()
    );
    assert!(
        eval("let b = new ArrayBuffer(4); let c = b.transferToImmutable(); c.resize(1);").is_err()
    );
    assert!(
        eval(
            "let b = new ArrayBuffer(4); let c = b.transferToImmutable(); c.transferToImmutable();"
        )
        .is_err()
    );
}

#[test]
fn detach_array_buffer_host_hook_marks_detached() {
    // The Test262 host hook detaches the buffer: byteLength reads 0 and the
    // typed-array view observes a detached buffer (methods throw).
    assert_eq!(
        eval("let b = new ArrayBuffer(8); __quickjsRustDetachArrayBuffer(b); b.byteLength;"),
        Ok(Value::Number(0.0))
    );
    assert!(
        eval(
            "let b = new ArrayBuffer(8); let a = new Uint8Array(b); \
             __quickjsRustDetachArrayBuffer(b); a.fill(1);"
        )
        .is_err()
    );
    // A non-ArrayBuffer argument is ignored and the hook returns null.
    assert_eq!(eval("__quickjsRustDetachArrayBuffer({});"), Ok(Value::Null));
}

#[test]
fn array_buffer_byte_length_brand_check() {
    assert!(
        eval("Object.getOwnPropertyDescriptor(ArrayBuffer.prototype, 'byteLength').get.call({});")
            .is_err()
    );
}
