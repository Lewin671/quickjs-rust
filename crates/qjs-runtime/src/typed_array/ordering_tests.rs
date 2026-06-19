use crate::{Value, eval};

#[test]
fn fill_writes_and_refreshes_reads() {
    // fill applies per-type conversion and keeps materialized index reads in
    // sync with the backing buffer.
    assert_eq!(
        eval(
            "let a = new Uint8Array(4); a.fill(257, 1, 3); a.join(',') + '|' + a[1] + ':' + a[3];"
        ),
        Ok(Value::String("0,1,1,0|1:0".to_owned().into()))
    );
}

#[test]
fn set_from_array_like_and_typed_array() {
    assert_eq!(
        eval("let a = new Uint8Array([0, 0, 0, 0]); a.set([10, 20], 1); a.join(',');"),
        Ok(Value::String("0,10,20,0".to_owned().into()))
    );
    assert_eq!(
        eval("let a = new Int16Array(3); a.set(new Uint8Array([5, 6])); a.join(',');"),
        Ok(Value::String("5,6,0".to_owned().into()))
    );
    // Out-of-range source throws RangeError.
    assert!(eval("new Uint8Array(2).set([1, 2, 3]);").is_err());
    assert_eq!(
        eval(
            "let caught = false; \
             try { new Uint8Array(2).set(new Uint8Array(2), Infinity); } \
             catch (e) { caught = e instanceof RangeError; } \
             caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; \
             try { new BigInt64Array(2).set(new BigInt64Array(2), Infinity); } \
             catch (e) { caught = e instanceof RangeError; } \
             caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // Negative offset throws RangeError.
    assert!(eval("new Uint8Array(4).set([1], -1);").is_err());
    // Mixing BigInt and Number typed arrays throws.
    assert!(eval("new BigInt64Array(2).set(new Uint8Array([1, 2]));").is_err());
}

#[test]
fn set_from_resizable_typed_array_validates_source_bounds() {
    assert_eq!(
        eval(
            "let buffer = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let fixed = new Uint8Array(buffer, 0, 4); \
             let tracking = new Uint8Array(buffer); \
             new Uint8Array(buffer).set([1, 2, 3, 4]); \
             let target = new Uint8Array(6); \
             buffer.resize(3); \
             let fixedThrew = false; \
             try { target.set(fixed); } catch (e) { fixedThrew = e instanceof TypeError; } \
             target.set(tracking, 1); \
             fixedThrew + ':' + target.join(',');"
        ),
        Ok(Value::String("true:0,1,2,3,0,0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let buffer = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let fixedOffset = new Uint8Array(buffer, 2, 2); \
             let trackingOffset = new Uint8Array(buffer, 2); \
             let target = new Uint8Array(4); \
             new Uint8Array(buffer).set([1, 2, 3, 4]); \
             buffer.resize(1); \
             let fixedThrew = false; \
             let trackingThrew = false; \
             try { target.set(fixedOffset); } catch (e) { fixedThrew = e instanceof TypeError; } \
             try { target.set(trackingOffset); } catch (e) { trackingThrew = e instanceof TypeError; } \
             fixedThrew + ':' + trackingThrew + ':' + target.join(',');"
        ),
        Ok(Value::String("true:true:0,0,0,0".to_owned().into()))
    );
}

#[test]
fn set_rejects_immutable_buffer_before_arguments() {
    assert_eq!(
        eval(
            "let calls = ''; \
             let target = new Uint8Array(new ArrayBuffer(4).transferToImmutable()); \
             let source = { get length() { calls += 'length'; return 1; }, get 0() { calls += 'value'; return 8; } }; \
             let offset = { valueOf() { calls += 'offset'; return 1; } }; \
             try { target.set(source, offset); } catch (e) { calls + ':' + (e instanceof TypeError) + ':' + target.join(','); }"
        ),
        Ok(Value::String(":true:0,0,0,0".to_owned().into()))
    );
}

#[test]
fn set_rechecks_target_after_offset_coercion() {
    assert_eq!(
        eval(
            "let target = new Uint8Array(2); \
             let offset = { valueOf() { __quickjsRustDetachArrayBuffer(target.buffer); return 0; } }; \
             try { target.set([1], offset); false; } catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let target = new Uint8Array(2); let source = new Uint8Array([1]); \
             let offset = { valueOf() { __quickjsRustDetachArrayBuffer(target.buffer); return 0; } }; \
             try { target.set(source, offset); false; } catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn set_from_array_like_writes_each_element_before_next_get() {
    assert_eq!(
        eval(
            "let target = new Uint8Array(5); \
             let seen = []; \
             let source = { \
               length: 3, \
               get 0() { seen.push(target.join(',')); return 42; }, \
               get 1() { seen.push(target.join(',')); return 43; }, \
               get 2() { seen.push(target.join(',')); return 44; } \
             }; \
             target.set(source, 1); \
             seen.join('|') + ';' + target.join(',');"
        ),
        Ok(Value::String(
            "0,0,0,0,0|0,42,0,0,0|0,42,43,0,0;0,42,43,44,0"
                .to_owned()
                .into()
        ))
    );
}

#[test]
fn set_from_array_like_preserves_prior_writes_on_abrupt_get() {
    assert_eq!(
        eval(
            "let target = new Uint8Array([1, 2, 3, 4]); \
             let source = { length: 4, 0: 42, 1: 43, get 2() { throw new Error('boom'); }, 3: 44 }; \
             let threw = false; \
             try { target.set(source); } catch (e) { threw = true; } \
             threw + ':' + target.join(',');"
        ),
        Ok(Value::String("true:42,43,3,4".to_owned().into()))
    );
}

#[test]
fn set_from_array_like_continues_after_target_detach() {
    assert_eq!(
        eval(
            "let target = new Uint8Array([1, 2, 3]); \
             let called = false; \
             let source = { length: 3, 0: 42, get 1() { __quickjsRustDetachArrayBuffer(target.buffer); }, get 2() { called = true; return 2; } }; \
             target.set(source); \
             called + ':' + target.length + ':' + target.byteLength + ':' + target.byteOffset;"
        ),
        Ok(Value::String("true:0:0:0".to_owned().into()))
    );
}

#[test]
fn copy_within_handles_overlap() {
    assert_eq!(
        eval("let a = new Uint8Array([1, 2, 3, 4, 5]); a.copyWithin(0, 3); a.join(',');"),
        Ok(Value::String("4,5,3,4,5".to_owned().into()))
    );
    assert_eq!(
        eval("let a = new Uint8Array([1, 2, 3, 4, 5]); a.copyWithin(1, 0, 2); a.join(',');"),
        Ok(Value::String("1,1,2,4,5".to_owned().into()))
    );
}

#[test]
fn copy_within_rejects_immutable_buffer_before_arguments() {
    assert_eq!(
        eval(
            "let calls = ''; \
             let target = new Uint8Array(new ArrayBuffer(4).transferToImmutable()); \
             let to = { valueOf() { calls += 'target'; return 1; } }; \
             let from = { valueOf() { calls += 'start'; return 2; } }; \
             let end = { valueOf() { calls += 'end'; return 3; } }; \
             try { target.copyWithin(to, from, end); } \
             catch (e) { calls + ':' + (e instanceof TypeError) + ':' + target.join(','); }"
        ),
        Ok(Value::String(":true:0,0,0,0".to_owned().into()))
    );
}

#[test]
fn copy_within_revalidates_after_argument_coercion() {
    assert_eq!(
        eval(
            "let target = new Uint8Array([1, 2, 3, 4]); \
             let start = { valueOf() { __quickjsRustDetachArrayBuffer(target.buffer); return 1; } }; \
             try { target.copyWithin(0, start, 3); false; } catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let target = new Uint8Array([1, 2, 3, 4]); \
             let end = { valueOf() { __quickjsRustDetachArrayBuffer(target.buffer); return 3; } }; \
             try { target.copyWithin(0, 1, end); false; } catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn copy_within_recomputes_resizable_view_length_after_coercion() {
    assert_eq!(
        eval(
            "let buffer = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let fixed = new Uint8Array(buffer, 0, 4); \
             fixed.set([0, 1, 2, 3]); \
             let target = { valueOf() { buffer.resize(2); return 1; } }; \
             try { fixed.copyWithin(target, 0); false; } catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let buffer = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let tracking = new Uint8Array(buffer); \
             tracking.set([0, 1, 2, 3]); \
             let start = { valueOf() { buffer.resize(3); return 2; } }; \
             tracking.copyWithin(0, start); \
             tracking.join(',');"
        ),
        Ok(Value::String("2,1,2".to_owned().into()))
    );
}

#[test]
fn copy_within_does_not_extend_copy_range_after_grow() {
    assert_eq!(
        eval(
            "let buffer = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let tracking = new Uint8Array(buffer); \
             tracking.set([0, 1, 2, 3]); \
             let target = { valueOf() { buffer.resize(6); tracking[4] = 4; tracking[5] = 5; return 0; } }; \
             tracking.copyWithin(target, 2); \
             tracking.join(',');"
        ),
        Ok(Value::String("2,3,2,3,4,5".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let buffer = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let tracking = new Uint8Array(buffer); \
             tracking.set([0, 1, 2, 3]); \
             let start = { valueOf() { buffer.resize(6); tracking[4] = 4; tracking[5] = 5; return 0; } }; \
             tracking.copyWithin(2, start); \
             tracking.join(',');"
        ),
        Ok(Value::String("0,1,0,1,4,5".to_owned().into()))
    );
}

#[test]
fn reverse_in_place_and_to_reversed_copies() {
    assert_eq!(
        eval(
            "let a = new Int8Array([1, 2, 3]); let r = a.reverse(); (r === a) + ':' + a.join(',');"
        ),
        Ok(Value::String("true:3,2,1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([1, 2, 3]); let r = a.toReversed(); a.join(',') + '|' + r.join(',');"
        ),
        Ok(Value::String("1,2,3|3,2,1".to_owned().into()))
    );
}

#[test]
fn sort_default_is_numeric_and_stable() {
    // Default ordering is numeric, not the string ordering used by Array.
    assert_eq!(
        eval("new Uint8Array([3, 20, 100, 1]).sort().join(',');"),
        Ok(Value::String("1,3,20,100".to_owned().into()))
    );
    // NaN sorts last, -0 before +0.
    assert_eq!(
        eval(
            "[...new Float64Array([NaN, 1, -0, 0, -1]).sort()].map(x => Object.is(x, -0) ? 'n0' : x).join(',');"
        ),
        Ok(Value::String("-1,n0,0,1,NaN".to_owned().into()))
    );
    // Comparator overrides ordering.
    assert_eq!(
        eval("new Uint8Array([1, 2, 3]).sort((a, b) => b - a).join(',');"),
        Ok(Value::String("3,2,1".to_owned().into()))
    );
}

#[test]
fn to_sorted_copies_and_with_replaces() {
    assert_eq!(
        eval(
            "let a = new Int16Array([3, 1, 2]); let r = a.toSorted(); a.join(',') + '|' + r.join(',');"
        ),
        Ok(Value::String("3,1,2|1,2,3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([1, 2, 3]); let r = a.with(1, 99); a.join(',') + '|' + r.join(',');"
        ),
        Ok(Value::String("1,2,3|1,99,3".to_owned().into()))
    );
    // Out-of-range index throws RangeError.
    assert!(eval("new Uint8Array(2).with(5, 1);").is_err());
}

#[test]
fn with_coerces_value_before_validating_current_index() {
    assert_eq!(
        eval(
            "function MyError() {} \
             let value = { valueOf() { throw new MyError(); } }; \
             try { new Uint8Array(1).with(100, value); false; } \
             catch (e) { e instanceof MyError; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let rab = new ArrayBuffer(2, { maxByteLength: 5 }); \
             let ta = new Int8Array(rab); \
             ta[0] = 11; ta[1] = 22; \
             let value = { valueOf() { rab.resize(5); return 123; } }; \
             let result = ta.with(4, value); \
             ta.length + ':' + result.length + ':' + result.join(',');"
        ),
        Ok(Value::String("5:2:11,22".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let rab = new ArrayBuffer(0, { maxByteLength: 1 }); \
             let ta = new Uint8Array(rab); \
             let value = { valueOf() { rab.resize(1); return 0; } }; \
             let result = ta.with(0, value); \
             result.length + ':' + rab.byteLength;"
        ),
        Ok(Value::String("0:1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let rab = new ArrayBuffer(4, { maxByteLength: 4 }); \
             let ta = new Uint8Array(rab); \
             let value = { valueOf() { rab.resize(1); return 123; } }; \
             try { ta.with(-1, value); false; } \
             catch (e) { (e instanceof RangeError) + ':' + rab.byteLength; }"
        ),
        Ok(Value::String("true:1".to_owned().into()))
    );
}

#[test]
fn bigint_fill_rejects_number() {
    assert!(eval("new BigInt64Array(2).fill(5);").is_err());
    assert_eq!(
        eval("new BigInt64Array(2).fill(5n).join(',');"),
        Ok(Value::String("5,5".to_owned().into()))
    );
    assert_eq!(
        eval(
            "try { new BigInt64Array(1).fill('nonsense'); false; } \
             catch (e) { e instanceof SyntaxError; }"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn fill_rechecks_buffer_after_argument_coercion() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4); let a = new Uint8Array(b); \
             let value = { valueOf() { __quickjsRustDetachArrayBuffer(b); return 7; } }; \
             try { a.fill(value, 0, 1); false; } catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4); let a = new Uint8Array(b); \
             let start = { valueOf() { __quickjsRustDetachArrayBuffer(b); return 0; } }; \
             try { a.fill(7, start, 1); false; } catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4); let a = new Uint8Array(b); \
             let end = { valueOf() { __quickjsRustDetachArrayBuffer(b); return 1; } }; \
             try { a.fill(7, 0, end); false; } catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); let a = new Uint8Array(b, 0, 4); \
             let value = { valueOf() { b.resize(2); return 7; } }; \
             try { a.fill(value, 0, 1); false; } catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn fill_rejects_immutable_buffer_before_argument_coercion() {
    assert_eq!(
        eval(
            "let calls = ''; let a = new Uint8Array(new ArrayBuffer(4).transferToImmutable()); \
             let value = { valueOf() { calls += 'value'; return 8; } }; \
             let start = { valueOf() { calls += 'start'; return 0; } }; \
             try { a.fill(value, start, 1); } catch (e) { calls + ':' + (e instanceof TypeError); }"
        ),
        Ok(Value::String(":true".to_owned().into()))
    );
}
