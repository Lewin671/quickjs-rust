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
fn array_buffer_slice_rejects_immutable_species_result() {
    assert_eq!(
        eval(
            "let calls = []; \
             let species = {}; \
             species[Symbol.species] = function(length) { calls.push('species:' + length); return new ArrayBuffer(8).transferToImmutable(); }; \
             let buffer = new ArrayBuffer(8); \
             buffer.constructor = species; \
             try { buffer.slice(); } catch (_) {} \
             calls.join('|');"
        ),
        Ok(Value::String("species:8".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = []; \
             let species = {}; \
             species[Symbol.species] = function(length) { calls.push('species:' + length); return new ArrayBuffer(8).transferToImmutable(); }; \
             let buffer = new ArrayBuffer(8); \
             buffer.constructor = species; \
             let start = { valueOf() { calls.push('start'); return 1; } }; \
             let end = { valueOf() { calls.push('end'); return 2; } }; \
             try { buffer.slice(start, end); } catch (_) {} \
             calls.join('|');"
        ),
        Ok(Value::String("start|end|species:1".to_owned()))
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
fn shared_array_buffer_slice_copies_bounds() {
    assert_eq!(
        eval(
            "let b = new SharedArrayBuffer(8); \
             let c = b.slice(2, 6); \
             [c instanceof SharedArrayBuffer, c.byteLength].join(':');"
        ),
        Ok(Value::String("true:4".to_owned()))
    );
    assert_eq!(
        eval("new SharedArrayBuffer(8).slice(-5, -1).byteLength;"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("new SharedArrayBuffer(8).slice(9, 1).byteLength;"),
        Ok(Value::Number(0.0))
    );
}

#[test]
fn shared_array_buffer_slice_uses_species_constructor() {
    assert_eq!(
        eval("SharedArrayBuffer[Symbol.species] === SharedArrayBuffer;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let result; \
             let species = {}; \
             species[Symbol.species] = function(length) { result = new SharedArrayBuffer(length + 2); return result; }; \
             let b = new SharedArrayBuffer(8); \
             b.constructor = species; \
             b.slice(1, 5) === result && result.byteLength === 6;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn shared_array_buffer_slice_validates_species_result() {
    assert!(
        eval(
            "let species = {}; \
             species[Symbol.species] = function() { return {}; }; \
             let b = new SharedArrayBuffer(8); \
             b.constructor = species; \
             b.slice();"
        )
        .is_err()
    );
    assert!(
        eval(
            "let species = {}; \
             species[Symbol.species] = function() { return new SharedArrayBuffer(4); }; \
             let b = new SharedArrayBuffer(8); \
             b.constructor = species; \
             b.slice();"
        )
        .is_err()
    );
    assert!(
        eval(
            "let species = {}; \
             let b = new SharedArrayBuffer(8); \
             species[Symbol.species] = function() { return b; }; \
             b.constructor = species; \
             b.slice();"
        )
        .is_err()
    );
    assert!(eval("SharedArrayBuffer.prototype.slice.call(new ArrayBuffer(8));").is_err());
}

#[test]
fn shared_array_buffer_growable_constructor_and_accessors() {
    assert_eq!(
        eval(
            "let b = new SharedArrayBuffer(4, { maxByteLength: 8 }); \
             [b.byteLength, b.maxByteLength, b.growable].join(':');"
        ),
        Ok(Value::String("4:8:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let b = new SharedArrayBuffer(4); \
             [b.byteLength, b.maxByteLength, b.growable].join(':');"
        ),
        Ok(Value::String("4:4:false".to_owned()))
    );
    assert_eq!(
        eval("new SharedArrayBuffer(0, null).growable;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("new SharedArrayBuffer(0, { maxByteLength: undefined }).growable;"),
        Ok(Value::Boolean(false))
    );
    assert!(eval("new SharedArrayBuffer(4, { maxByteLength: 3 });").is_err());
    assert!(eval("new SharedArrayBuffer(0, { maxByteLength: 1000001 });").is_err());
}

#[test]
fn shared_array_buffer_allocation_checks_after_object_creation() {
    assert_eq!(
        eval(
            "function DummyError() {} \
             let newTarget = Object.defineProperty(function(){}.bind(null), 'prototype', { \
               get() { throw new DummyError(); } \
             }); \
             try { \
               Reflect.construct(SharedArrayBuffer, [7 * 1125899906842624], newTarget); \
               false; \
             } catch (error) { \
               error instanceof DummyError; \
             }"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn shared_array_buffer_grow_resizes_within_max_length() {
    assert_eq!(
        eval(
            "let b = new SharedArrayBuffer(4, { maxByteLength: 6 }); \
             let result = b.grow(6); \
             [result, b.byteLength, b.maxByteLength, b.growable].join(':');"
        ),
        Ok(Value::String(":6:6:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let b = new SharedArrayBuffer(4, { maxByteLength: 6 }); \
             b.grow(4); b.byteLength;"
        ),
        Ok(Value::Number(4.0))
    );
    assert!(eval("let b = new SharedArrayBuffer(4); b.grow(4);").is_err());
    assert!(eval("let b = new SharedArrayBuffer(4, { maxByteLength: 6 }); b.grow(3);").is_err());
    assert!(eval("let b = new SharedArrayBuffer(4, { maxByteLength: 6 }); b.grow(7);").is_err());
    assert!(eval("SharedArrayBuffer.prototype.grow.call(new ArrayBuffer(0));").is_err());
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
fn array_buffer_transfer_copies_detaches_and_preserves_resizability() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let a = new Uint8Array(b); a[0] = 7; a[1] = 8; \
             let c = b.transfer(6); \
             [b.byteLength, b.detached, c.byteLength, c.resizable, c.maxByteLength, new Uint8Array(c)[0], new Uint8Array(c)[1], new Uint8Array(c)[5]].join(':');"
        ),
        Ok(Value::String("0:true:6:true:8:7:8:0".to_owned()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4); \
             let a = new Uint8Array(b); a[0] = 3; a[1] = 4; \
             let c = b.transfer(2); \
             [b.byteLength, c.byteLength, c.resizable, c.maxByteLength, new Uint8Array(c)[0], new Uint8Array(c)[1]].join(':');"
        ),
        Ok(Value::String("0:2:false:2:3:4".to_owned()))
    );
}

#[test]
fn array_buffer_transfer_to_fixed_length_drops_resizability() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let a = new Uint8Array(b); a[0] = 1; a[1] = 2; \
             let c = b.transferToFixedLength(5); \
             [b.byteLength, b.detached, c.byteLength, c.resizable, c.maxByteLength, new Uint8Array(c)[0], new Uint8Array(c)[1], new Uint8Array(c)[4]].join(':');"
        ),
        Ok(Value::String("0:true:5:false:5:1:2:0".to_owned()))
    );
}

#[test]
fn array_buffer_transfer_coerces_length_before_detached_and_immutable_checks() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4); \
             __quickjsRustDetachArrayBuffer(b); \
             let calls = 0; \
             try { b.transfer({ valueOf() { calls++; return 1; } }); } catch (_) {} \
             calls;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4).transferToImmutable(); \
             let calls = 0; \
             try { b.transferToFixedLength({ valueOf() { calls++; return 1; } }); } catch (_) {} \
             calls;"
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
