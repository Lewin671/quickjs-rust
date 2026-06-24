use crate::{Value, eval};

#[path = "indexed_tests.rs"]
mod indexed_tests;

// --- Intrinsic and brand -----------------------------------------------------

#[test]
fn typed_array_intrinsic_is_shared_prototype() {
    assert_eq!(
        eval("Object.getPrototypeOf(Uint8Array) === Object.getPrototypeOf(Int8Array);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "Object.getPrototypeOf(Uint8Array.prototype) === Object.getPrototypeOf(Int8Array.prototype);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getPrototypeOf(Object.getPrototypeOf(Uint8Array)) === Function.prototype;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn typed_array_intrinsic_not_directly_callable_or_constructable() {
    let intrinsic = "Object.getPrototypeOf(Uint8Array)";
    assert!(eval(&format!("new ({intrinsic})();")).is_err());
    assert!(eval(&format!("({intrinsic})();")).is_err());
}

#[test]
fn concrete_constructor_requires_new() {
    assert!(eval("Uint8Array(3);").is_err());
}

// --- Construction variants ---------------------------------------------------

#[test]
fn construct_from_length_and_no_args() {
    assert_eq!(
        eval("let a = new Float64Array(3); a.length + ':' + a[0] + ':' + a[2];"),
        Ok(Value::String("3:0:0".to_owned().into()))
    );
    assert_eq!(eval("new Uint8Array().length;"), Ok(Value::Number(0.0)));
}

#[test]
fn construct_from_length_uses_to_index_and_new_target_prototype() {
    assert_eq!(
        eval(
            "let negative = false; \
             try { new Uint8Array(-1); } catch (e) { negative = e instanceof RangeError; } \
             let symbol = false; \
             try { new Uint8Array(Symbol('1')); } catch (e) { symbol = e instanceof TypeError; } \
             negative + ':' + symbol;"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let marker = {}; \
             let newTarget = function() {}.bind(null); \
             Object.defineProperty(newTarget, 'prototype', { get() { throw marker; } }); \
             let caught = false; \
             try { Reflect.construct(Uint8Array, [1], newTarget); } catch (e) { caught = e === marker; } \
             caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let proto = { tag: 'typed' }; \
             function NewTarget() {} \
             NewTarget.prototype = proto; \
             Object.getPrototypeOf(Reflect.construct(Int16Array, [1], NewTarget)) === proto;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn instances_inherit_length_accessor_and_allow_own_length() {
    assert_eq!(
        eval("let a = new Uint8Array([7]); a.length + ':' + a.hasOwnProperty('length');"),
        Ok(Value::String("1:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([7]); Object.defineProperty(a, 'length', { value: 3 }); a[Symbol.isConcatSpreadable] = true; let out = [].concat(a); out.length + ':' + out[0] + ':' + out.hasOwnProperty('1') + ':' + out.hasOwnProperty('2');"
        ),
        Ok(Value::String("3:7:false:false".to_owned().into()))
    );
}

#[test]
fn construct_from_array_like_and_iterable() {
    assert_eq!(
        eval("let a = new Uint8Array([1, 258]); a.length + ':' + a[0] + ':' + a[1];"),
        Ok(Value::String("2:1:2".to_owned().into()))
    );
    assert_eq!(
        eval("let a = new Int8Array([127, 128, 255]); a[0] + ':' + a[1] + ':' + a[2];"),
        Ok(Value::String("127:-128:-1".to_owned().into()))
    );
    // Iterable (a Set) is consumed through Symbol.iterator.
    assert_eq!(
        eval("let a = new Uint16Array(new Set([7, 8, 7])); a.length + ':' + a[0] + ':' + a[1];"),
        Ok(Value::String("2:7:8".to_owned().into()))
    );
}

#[test]
fn construct_from_array_observes_iterator_overrides_and_prototype_indices() {
    assert_eq!(
        eval(
            "let original = Array.prototype[Symbol.iterator]; \
             Array.prototype[Symbol.iterator] = function() { return original.call([9]); }; \
             let out; \
             try { let a = new Uint8Array([1, 2]); out = a.length + ':' + a[0]; } \
             finally { Array.prototype[Symbol.iterator] = original; } \
             out;"
        ),
        Ok(Value::String("1:9".to_owned().into()))
    );
    assert_eq!(
        eval(
            "Object.defineProperty(Array.prototype, '0', { get() { return 7; }, configurable: true }); \
             let out; \
             try { let a = new Uint8Array([, 3]); out = a[0] + ':' + a[1]; } \
             finally { delete Array.prototype[0]; } \
             out;"
        ),
        Ok(Value::String("7:3".to_owned().into()))
    );
}

#[test]
fn construct_from_array_like_observes_accessors_and_inherited_indices() {
    assert_eq!(
        eval(
            "let calls = 0; \
             let source = { length: 2, 1: 3 }; \
             Object.defineProperty(source, '0', { get() { calls++; return 7; } }); \
             let a = new Uint8Array(source); \
             calls + ':' + a[0] + ':' + a[1];"
        ),
        Ok(Value::String("1:7:3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let proto = { 0: 9 }; \
             let source = { length: 2, 1: 4 }; \
             Object.setPrototypeOf(source, proto); \
             let a = new Uint8Array(source); \
             a[0] + ':' + a[1];"
        ),
        Ok(Value::String("9:4".to_owned().into()))
    );
}

#[test]
fn construct_from_array_like_rejects_excessive_length_before_index_reads() {
    assert_eq!(
        eval(
            "let reads = 0; \
             let source = { length: Math.pow(2, 53) }; \
             Object.defineProperty(source, '0', { get() { reads++; throw new TypeError('index'); } }); \
             let threw = false; \
             try { new Uint8Array(source); } catch (e) { threw = e instanceof RangeError; } \
             threw + ':' + reads;"
        ),
        Ok(Value::String("true:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let source = { length: Math.pow(2, 53), 0: 1n }; \
             let threw = false; \
             try { new BigInt64Array(source); } catch (e) { threw = e instanceof RangeError; } \
             threw;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let source = { length: Math.pow(2, 53), [Symbol.iterator]: function() { return [7, 8][Symbol.iterator](); } }; \
             let a = new Uint8Array(source); \
             a.length + ':' + a[0] + ':' + a[1];"
        ),
        Ok(Value::String("2:7:8".to_owned().into()))
    );
}

#[test]
fn construct_from_typed_array_converts_elements() {
    assert_eq!(
        eval(
            "let s = new Uint8Array([1, 2, 3]); let c = new Int16Array(s); c.length + ':' + c[0] + ':' + c[2];"
        ),
        Ok(Value::String("3:1:3".to_owned().into()))
    );
}

#[test]
fn construct_from_buffer_with_offset_and_length() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(8); let a = new Uint8Array(b, 2, 4); a.length + ':' + a.byteOffset + ':' + a.byteLength;"
        ),
        Ok(Value::String("4:2:4".to_owned().into()))
    );
    // Default length covers the rest of the buffer.
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(8); let a = new Uint32Array(b, 4); a.length + ':' + a.byteOffset;"
        ),
        Ok(Value::String("1:4".to_owned().into()))
    );
}

#[test]
fn resizable_buffer_views_track_effective_length() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(8, { maxByteLength: 16 }); \
             let fixed = new Uint8Array(b, 0, 4); \
             let tracking = new Uint8Array(b, 2); \
             b.resize(3); \
             fixed.length + ':' + fixed.byteLength + ':' + fixed.byteOffset + '|' \
             + tracking.length + ':' + tracking.byteLength + ':' + tracking.byteOffset;"
        ),
        Ok(Value::String("0:0:0|1:1:2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(8, { maxByteLength: 16 }); \
             let a = new Uint8Array(b); a[0] = 7; b.resize(2); b.resize(4); \
             a.length + ':' + a[0] + ':' + a[2];"
        ),
        Ok(Value::String("4:7:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(10, { maxByteLength: 20 }); \
             let a = new Float64Array(b); \
             a.length + ':' + a.byteLength;"
        ),
        Ok(Value::String("1:8".to_owned().into()))
    );
}

#[test]
fn construct_from_resizable_typed_array_uses_current_bounds() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let fixed = new Uint8Array(b, 0, 4); \
             let fixedOffset = new Uint8Array(b, 2, 2); \
             let tracking = new Uint8Array(b); \
             let trackingOffset = new Uint8Array(b, 2); \
             let full = new Uint8Array(b); full.set([1, 2, 3, 4]); \
             b.resize(3); \
             let fixedThrew = false; \
             try { new Uint8Array(fixed); } catch (e) { fixedThrew = e instanceof TypeError; } \
             [fixedThrew, Array.from(new Uint8Array(tracking)).join(','), \
              Array.from(new Uint8Array(trackingOffset)).join(',')].join('|');"
        ),
        Ok(Value::String("true|1,2,3|3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let fixedOffset = new Uint8Array(b, 2, 2); \
             let trackingOffset = new Uint8Array(b, 2); \
             b.resize(1); \
             let fixedThrew = false; \
             let trackingThrew = false; \
             try { new Uint8Array(fixedOffset); } catch (e) { fixedThrew = e instanceof TypeError; } \
             try { new Uint8Array(trackingOffset); } catch (e) { trackingThrew = e instanceof TypeError; } \
             fixedThrew + ':' + trackingThrew;"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn array_copy_within_uses_resizable_typed_array_elements() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let a = new Uint8Array(b); a.set([0, 1, 2, 3]); \
             Array.prototype.copyWithin.call(a, 1, 2); \
             Array.prototype.join.call(a);"
        ),
        Ok(Value::String("0,2,3,3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let fixed = new Uint8Array(b, 0, 4); fixed.set([0, 1, 2, 3]); \
             b.resize(2); Array.prototype.copyWithin.call(fixed, 0, 1); \
             fixed.length + ':' + fixed[0] + ':' + Array.prototype.join.call(new Uint8Array(b));"
        ),
        Ok(Value::String("0:undefined:0,1".to_owned().into()))
    );
}

#[test]
fn array_prototype_iterators_validate_resizable_view_on_next() {
    assert_eq!(
        eval(
            "let reads = 0; \
             let object = { get length() { reads++; return 1; }, 0: 7 }; \
             let iterator = Array.prototype.values.call(object); \
             let before = reads; \
             iterator.next(); \
             before + ':' + reads;"
        ),
        Ok(Value::String("0:1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let a = new Uint8Array(b, 0, 4); \
             b.resize(3); \
             let created = false; \
             try { Array.prototype.entries.call(a); created = true; } catch (e) {} \
             let threw = false; \
             try { Array.from(Array.prototype.entries.call(a)); } catch (e) { threw = e instanceof TypeError; } \
             created + ':' + threw;"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn construct_from_buffer_validates_alignment_and_bounds() {
    // Misaligned offset.
    assert!(eval("new Uint32Array(new ArrayBuffer(8), 2);").is_err());
    // Length out of range.
    assert!(eval("new Uint8Array(new ArrayBuffer(4), 0, 8);").is_err());
    // Buffer not aligned to element size with implicit length.
    assert!(eval("new Uint32Array(new ArrayBuffer(6));").is_err());
}

// --- Static methods: from / of ----------------------------------------------

#[test]
fn typed_array_of_constructs_and_coerces() {
    assert_eq!(
        eval("let a = Uint8Array.of(5, 6, 257); a.length + ':' + a[0] + ':' + a[2];"),
        Ok(Value::String("3:5:1".to_owned().into()))
    );
    // `of` is shared across the concrete constructors via %TypedArray%.
    assert_eq!(
        eval("Uint8Array.of === Int8Array.of;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let a = BigInt64Array.of(1n, 2n); \
             typeof a[0] + ':' + a[1] + ':' + (BigInt64Array.of === Uint8Array.of) + ':' \
             + BigInt64Array.hasOwnProperty('of');"
        ),
        Ok(Value::String("bigint:2:true:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let custom = new BigInt64Array(3); \
             let result = BigInt64Array.of.call(function() { return custom; }, 1n, 2n); \
             (result === custom) + ':' + custom[0] + ':' + custom[1] + ':' + custom[2];"
        ),
        Ok(Value::String("true:1:2:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let threw = false; \
             try { BigInt64Array.of.call(function() { return {}; }, 1n); } \
             catch (e) { threw = e instanceof TypeError; } \
             threw;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let threw = false; \
             try { BigInt64Array.of.call(function() { return new BigInt64Array(1); }, 1n, 2n); } \
             catch (e) { threw = e instanceof TypeError; } \
             threw;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let marker = {}; let last = ''; \
             let first = { valueOf() { last = 'first'; return 1n; } }; \
             let second = { valueOf() { last = 'second'; throw marker; } }; \
             let caught = false; \
             try { BigInt64Array.of(first, second, first); } \
             catch (e) { caught = e === marker; } \
             caught + ':' + last;"
        ),
        Ok(Value::String("true:second".to_owned().into()))
    );
}

#[test]
fn typed_array_from_iterable_array_like_and_mapfn() {
    assert_eq!(
        eval("let a = Uint8Array.from([1, 2, 300]); a.length + ':' + a[0] + ':' + a[2];"),
        Ok(Value::String("3:1:44".to_owned().into()))
    );
    // Array-like (no Symbol.iterator) source.
    assert_eq!(
        eval("let a = Int8Array.from({ length: 2, 0: 10, 1: 20 }); a[0] + ':' + a[1];"),
        Ok(Value::String("10:20".to_owned().into()))
    );
    // mapfn receives (value, index).
    assert_eq!(
        eval("let a = Int8Array.from([1, 2, 3], (v, i) => v * 10 + i); a[0] + ':' + a[2];"),
        Ok(Value::String("10:32".to_owned().into()))
    );
    // A non-callable mapfn throws.
    assert!(eval("Uint8Array.from([1], 5);").is_err());
    // BigInt arrays round-trip BigInt elements.
    assert_eq!(
        eval("let a = BigInt64Array.from([1n, 2n]); typeof a[0] + ':' + a[1];"),
        Ok(Value::String("bigint:2".to_owned().into()))
    );
}

// --- Accessors and brand checks ----------------------------------------------

#[test]
fn instance_accessors_report_view_geometry() {
    assert_eq!(
        eval(
            "let a = new Int16Array(3); a.buffer.byteLength + ':' + a.byteLength + ':' + a.byteOffset + ':' + a.length;"
        ),
        Ok(Value::String("6:6:0:3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array(1); Object.getOwnPropertyDescriptor(a, 'length') === undefined;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array(1); Object.defineProperty(a, 'length', { value: 4 }); a.length;"
        ),
        Ok(Value::Number(4.0))
    );
}

#[test]
fn accessors_brand_check_their_receiver() {
    assert!(
        eval("Object.getOwnPropertyDescriptor(Object.getPrototypeOf(Uint8Array.prototype), 'byteLength').get.call({});").is_err()
    );
    // Symbol.toStringTag accessor returns undefined (does not throw) off-brand.
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(Uint8Array.prototype), Symbol.toStringTag); d.get.call({});"
        ),
        Ok(Value::Undefined)
    );
}

#[test]
fn object_to_string_reports_kind() {
    assert_eq!(
        eval("Object.prototype.toString.call(new Uint8ClampedArray(0));"),
        Ok(Value::String(
            "[object Uint8ClampedArray]".to_owned().into()
        ))
    );
}

// --- Element conversion ------------------------------------------------------

#[test]
fn uint8_clamped_clamps_and_rounds_half_even() {
    assert_eq!(
        eval("Array.prototype.join.call(new Uint8ClampedArray([-1, 2.5, 3.5, 300]));"),
        Ok(Value::String("0,2,4,255".to_owned().into()))
    );
}

#[test]
fn bigint_arrays_wrap_and_reject_numbers() {
    assert_eq!(
        eval("let a = new BigInt64Array([1n, 2n]); typeof a[0] + ':' + a[0] + ':' + a[1];"),
        Ok(Value::String("bigint:1:2".to_owned().into()))
    );
    assert_eq!(
        eval("let a = new BigInt64Array(['0', true]); typeof a[0] + ':' + a[0] + ':' + a[1];"),
        Ok(Value::String("bigint:0:1".to_owned().into()))
    );
    assert!(eval("new BigInt64Array([1]);").is_err());
}

#[test]
fn object_constructor_rejects_non_callable_iterator_method() {
    assert!(
        eval(
            "let source = { length: 1, 0: 7 }; source[Symbol.iterator] = 1; new Uint8Array(source);"
        )
        .is_err()
    );
    assert!(
        eval("let source = { length: 1, 0: 7n }; source[Symbol.iterator] = true; new BigInt64Array(source);")
            .is_err()
    );
    assert_eq!(
        eval(
            "let source = { length: 1, 0: 7 }; source[Symbol.iterator] = null; new Uint8Array(source)[0];"
        ),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn bytes_per_element_surface() {
    assert_eq!(
        eval("Int16Array.BYTES_PER_ELEMENT + ':' + Int32Array.prototype.BYTES_PER_ELEMENT;"),
        Ok(Value::String("2:4".to_owned().into()))
    );
}

#[test]
fn uint8_array_to_base64_encodes_with_options() {
    assert_eq!(
        eval(
            "[
               new Uint8Array([]).toBase64(),
               new Uint8Array([102]).toBase64(),
               new Uint8Array([102, 111]).toBase64(),
               new Uint8Array([102, 111, 111]).toBase64(),
               new Uint8Array([199, 239, 242]).toBase64({ alphabet: 'base64url' }),
               new Uint8Array([255]).toBase64({ omitPadding: true }),
               new Uint8Array([255]).toBase64({ alphabet: 'base64url', omitPadding: true })
             ].join('|');"
        ),
        Ok(Value::String(
            "|Zg==|Zm8=|Zm9v|x-_y|/w|_w".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([0]); \
             let calls = 0; \
             let options = {}; \
             Object.defineProperty(options, 'alphabet', { get() { calls++; a[0] = 255; return 'base64'; } }); \
             a.toBase64(options) + ':' + calls + ':' + a[0];"
        ),
        Ok(Value::String("/w==:1:255".to_owned().into()))
    );
}

#[test]
fn uint8_array_to_base64_surface_and_errors() {
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Uint8Array.prototype, 'toBase64'); \
             d.writable + ':' + d.enumerable + ':' + d.configurable + ':' \
             + Uint8Array.prototype.toBase64.name + ':' \
             + Uint8Array.prototype.toBase64.length;"
        ),
        Ok(Value::String(
            "true:false:true:toBase64:0".to_owned().into()
        ))
    );
    assert!(eval("new Uint8Array.prototype.toBase64();").is_err());
    assert!(eval("Uint8Array.prototype.toBase64.call(new Int8Array(1));").is_err());
    assert!(eval("new Uint8Array([1]).toBase64({ alphabet: 'other' });").is_err());
    assert!(eval("new Uint8Array([1]).toBase64({ alphabet: Object('base64') });").is_err());
}

#[test]
fn uint8_array_to_base64_checks_detached_after_options() {
    assert_eq!(
        eval(
            "let a = new Uint8Array(2); \
             let calls = 0; \
             let options = {}; \
             Object.defineProperty(options, 'alphabet', { get() { calls++; __quickjsRustDetachArrayBuffer(a.buffer); return 'base64'; } }); \
             let threw = false; \
             try { a.toBase64(options); } catch (e) { threw = e instanceof TypeError; } \
             threw + ':' + calls;"
        ),
        Ok(Value::String("true:1".to_owned().into()))
    );
}

#[test]
fn sort_rejects_immutable_buffer_before_comparing() {
    assert_eq!(
        eval(
            "let calls = ''; \
             let a = new Uint8Array(new ArrayBuffer(4).transferToImmutable()); \
             try { a.sort(() => { calls += 'compare'; return 0; }); } \
             catch (e) { calls + ':' + (e instanceof TypeError) + ':' + a.length; }"
        ),
        Ok(Value::String(":true:4".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let calls = ''; \
             let a = new Uint8Array(new ArrayBuffer(0).transferToImmutable()); \
             try { a.sort(() => { calls += 'compare'; return 0; }); } \
             catch (e) { calls + ':' + (e instanceof TypeError) + ':' + a.length; }"
        ),
        Ok(Value::String(":true:0".to_owned().into()))
    );
}

// --- Batched element materialization (perf slice) ----------------------------

#[test]
fn batched_writes_stay_correct_across_a_full_buffer() {
    // Exercises the single-pass byte encode/decode for fill, set, copyWithin,
    // sort, and reverse over a larger buffer; the materialized index reads must
    // match the backing buffer at both ends and an interior index.
    assert_eq!(
        eval(
            "let a = new Uint16Array(256); a.fill(7); \
             a.set([1, 2, 3], 10); a.copyWithin(20, 10, 13); \
             a[0] + ':' + a[10] + ':' + a[22] + ':' + a[255];"
        ),
        Ok(Value::String("7:1:3:7".to_owned().into()))
    );
    // A reversed view reads back consistently through materialized properties.
    assert_eq!(
        eval(
            "let a = new Int32Array(100); for (let i = 0; i < 100; i++) a.fill(0); \
             let b = a.map((_, i) => i); b.reverse(); b[0] + ':' + b[99];"
        ),
        Ok(Value::String("99:0".to_owned().into()))
    );
}

#[test]
fn callback_iteration_reads_each_element_from_the_buffer() {
    // The snapshot reader feeds every element to the callback in order; reducing
    // over a larger view sums all of them, confirming reads stay correct after
    // the single up-front byte decode.
    assert_eq!(
        eval(
            "let a = new Uint16Array(64); a.map((_, i) => i); \
             let b = new Uint16Array(64).map((_, i) => i + 1); \
             b.reduce((p, c) => p + c, 0);"
        ),
        Ok(Value::Number(2080.0))
    );
    // forEach observes index and value together for the whole length.
    assert_eq!(
        eval(
            "let a = new Int8Array([10, 20, 30]); let parts = []; \
             a.forEach((v, i) => parts.push(i + '=' + v)); parts.join(',');"
        ),
        Ok(Value::String("0=10,1=20,2=30".to_owned().into()))
    );
    // Callback-driven reads must observe writes made by earlier callbacks.
    assert_eq!(
        eval(
            "let a = new Int8Array([42, 43, 44]); let seen = []; \
             a.forEach((v, i) => { seen.push(v); if (i < a.length - 1) a[i + 1] = 42; }); \
             seen.join(',');"
        ),
        Ok(Value::String("42,42,42".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = new BigInt64Array([42n, 43n, 44n]); let seen = []; \
             a.forEach((v, i) => { seen.push(String(v)); if (i < a.length - 1) a[i + 1] = 42n; }); \
             seen.join(',');"
        ),
        Ok(Value::String("42,42,42".to_owned().into()))
    );
}
