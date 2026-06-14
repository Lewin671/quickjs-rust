use crate::{Value, eval};

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
        Ok(Value::String("3:0:0".to_owned()))
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
        Ok(Value::String("true:true".to_owned()))
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
        Ok(Value::String("1:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([7]); Object.defineProperty(a, 'length', { value: 3 }); a[Symbol.isConcatSpreadable] = true; let out = [].concat(a); out.length + ':' + out[0] + ':' + out.hasOwnProperty('1') + ':' + out.hasOwnProperty('2');"
        ),
        Ok(Value::String("3:7:false:false".to_owned()))
    );
}

#[test]
fn construct_from_array_like_and_iterable() {
    assert_eq!(
        eval("let a = new Uint8Array([1, 258]); a.length + ':' + a[0] + ':' + a[1];"),
        Ok(Value::String("2:1:2".to_owned()))
    );
    assert_eq!(
        eval("let a = new Int8Array([127, 128, 255]); a[0] + ':' + a[1] + ':' + a[2];"),
        Ok(Value::String("127:-128:-1".to_owned()))
    );
    // Iterable (a Set) is consumed through Symbol.iterator.
    assert_eq!(
        eval("let a = new Uint16Array(new Set([7, 8, 7])); a.length + ':' + a[0] + ':' + a[1];"),
        Ok(Value::String("2:7:8".to_owned()))
    );
}

#[test]
fn construct_from_typed_array_converts_elements() {
    assert_eq!(
        eval(
            "let s = new Uint8Array([1, 2, 3]); let c = new Int16Array(s); c.length + ':' + c[0] + ':' + c[2];"
        ),
        Ok(Value::String("3:1:3".to_owned()))
    );
}

#[test]
fn construct_from_buffer_with_offset_and_length() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(8); let a = new Uint8Array(b, 2, 4); a.length + ':' + a.byteOffset + ':' + a.byteLength;"
        ),
        Ok(Value::String("4:2:4".to_owned()))
    );
    // Default length covers the rest of the buffer.
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(8); let a = new Uint32Array(b, 4); a.length + ':' + a.byteOffset;"
        ),
        Ok(Value::String("1:4".to_owned()))
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
        Ok(Value::String("0:0:0|1:1:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(8, { maxByteLength: 16 }); \
             let a = new Uint8Array(b); a[0] = 7; b.resize(2); b.resize(4); \
             a.length + ':' + a[0] + ':' + a[2];"
        ),
        Ok(Value::String("4:7:0".to_owned()))
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
        Ok(Value::String("0,2,3,3".to_owned()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let fixed = new Uint8Array(b, 0, 4); fixed.set([0, 1, 2, 3]); \
             b.resize(2); Array.prototype.copyWithin.call(fixed, 0, 1); \
             fixed.length + ':' + fixed[0] + ':' + Array.prototype.join.call(new Uint8Array(b));"
        ),
        Ok(Value::String("0:undefined:0,1".to_owned()))
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
        Ok(Value::String("0:1".to_owned()))
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
        Ok(Value::String("true:true".to_owned()))
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
        Ok(Value::String("3:5:1".to_owned()))
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
        Ok(Value::String("bigint:2:true:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let custom = new BigInt64Array(3); \
             let result = BigInt64Array.of.call(function() { return custom; }, 1n, 2n); \
             (result === custom) + ':' + custom[0] + ':' + custom[1] + ':' + custom[2];"
        ),
        Ok(Value::String("true:1:2:0".to_owned()))
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
        Ok(Value::String("true:second".to_owned()))
    );
}

#[test]
fn typed_array_from_iterable_array_like_and_mapfn() {
    assert_eq!(
        eval("let a = Uint8Array.from([1, 2, 300]); a.length + ':' + a[0] + ':' + a[2];"),
        Ok(Value::String("3:1:44".to_owned()))
    );
    // Array-like (no Symbol.iterator) source.
    assert_eq!(
        eval("let a = Int8Array.from({ length: 2, 0: 10, 1: 20 }); a[0] + ':' + a[1];"),
        Ok(Value::String("10:20".to_owned()))
    );
    // mapfn receives (value, index).
    assert_eq!(
        eval("let a = Int8Array.from([1, 2, 3], (v, i) => v * 10 + i); a[0] + ':' + a[2];"),
        Ok(Value::String("10:32".to_owned()))
    );
    // A non-callable mapfn throws.
    assert!(eval("Uint8Array.from([1], 5);").is_err());
    // BigInt arrays round-trip BigInt elements.
    assert_eq!(
        eval("let a = BigInt64Array.from([1n, 2n]); typeof a[0] + ':' + a[1];"),
        Ok(Value::String("bigint:2".to_owned()))
    );
}

// --- Accessors and brand checks ----------------------------------------------

#[test]
fn instance_accessors_report_view_geometry() {
    assert_eq!(
        eval(
            "let a = new Int16Array(3); a.buffer.byteLength + ':' + a.byteLength + ':' + a.byteOffset + ':' + a.length;"
        ),
        Ok(Value::String("6:6:0:3".to_owned()))
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
        Ok(Value::String("[object Uint8ClampedArray]".to_owned()))
    );
}

// --- Element conversion ------------------------------------------------------

#[test]
fn uint8_clamped_clamps_and_rounds_half_even() {
    assert_eq!(
        eval("Array.prototype.join.call(new Uint8ClampedArray([-1, 2.5, 3.5, 300]));"),
        Ok(Value::String("0,2,4,255".to_owned()))
    );
}

#[test]
fn bigint_arrays_wrap_and_reject_numbers() {
    assert_eq!(
        eval("let a = new BigInt64Array([1n, 2n]); typeof a[0] + ':' + a[0] + ':' + a[1];"),
        Ok(Value::String("bigint:1:2".to_owned()))
    );
    assert_eq!(
        eval("let a = new BigInt64Array(['0', true]); typeof a[0] + ':' + a[0] + ':' + a[1];"),
        Ok(Value::String("bigint:0:1".to_owned()))
    );
    assert!(eval("new BigInt64Array([1]);").is_err());
}

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
fn bytes_per_element_surface() {
    assert_eq!(
        eval("Int16Array.BYTES_PER_ELEMENT + ':' + Int32Array.prototype.BYTES_PER_ELEMENT;"),
        Ok(Value::String("2:4".to_owned()))
    );
}

// --- Prototype methods: iteration / read family (batch 1) --------------------

#[test]
fn prototype_methods_are_shared_and_brand_checked() {
    // Methods live on the shared %TypedArray.prototype%, not the concrete one.
    assert_eq!(
        eval("Uint8Array.prototype.hasOwnProperty('map');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Object.getPrototypeOf(Uint8Array.prototype).hasOwnProperty('map');"),
        Ok(Value::Boolean(true))
    );
    // Off-brand receiver throws.
    assert!(eval("Uint8Array.prototype.join.call({});").is_err());
}

#[test]
fn at_and_includes_and_index_of() {
    assert_eq!(
        eval("let a = new Int16Array([5, 10, 15]); a.at(-1) + ':' + a.at(0) + ':' + a.at(5);"),
        Ok(Value::String("15:5:undefined".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([1, 2, 3]); a.indexOf(2) + ':' + a.indexOf(9) + ':' + a.lastIndexOf(3);"
        ),
        Ok(Value::String("1:-1:2".to_owned()))
    );
    assert_eq!(
        eval("new Float64Array([NaN]).includes(NaN);"),
        Ok(Value::Boolean(true))
    );
    // indexOf uses strict equality, so NaN is never found.
    assert_eq!(
        eval("new Float64Array([NaN]).indexOf(NaN);"),
        Ok(Value::Number(-1.0))
    );
}

#[test]
fn join_and_to_string() {
    assert_eq!(
        eval("new Uint8Array([1, 2, 3]).join('-');"),
        Ok(Value::String("1-2-3".to_owned()))
    );
    assert_eq!(
        eval("new Uint8Array([1, 2, 3]).toString();"),
        Ok(Value::String("1,2,3".to_owned()))
    );
    assert_eq!(
        eval("Object.getPrototypeOf(Uint8Array.prototype).toString === Array.prototype.toString;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn iterators_keys_values_entries() {
    assert_eq!(
        eval("Array.from(new Uint8Array([7, 8]).keys()).join(',');"),
        Ok(Value::String("0,1".to_owned()))
    );
    assert_eq!(
        eval("[...new Uint8Array([7, 8])].join(',');"),
        Ok(Value::String("7,8".to_owned()))
    );
    assert_eq!(
        eval(
            "let e = [...new Uint8Array([7, 8]).entries()]; e[0].join(':') + '|' + e[1].join(':');"
        ),
        Ok(Value::String("0:7|1:8".to_owned()))
    );
    // Symbol.iterator is the same function object as values.
    assert_eq!(
        eval(
            "Object.getPrototypeOf(Uint8Array.prototype)[Symbol.iterator] === Object.getPrototypeOf(Uint8Array.prototype).values;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "Object.getPrototypeOf(new Uint8Array([1]).values()) === Object.getPrototypeOf([][Symbol.iterator]());"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "Object.getPrototypeOf(new BigInt64Array([1n]).values()) === Object.getPrototypeOf([][Symbol.iterator]());"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let b = BigInt64Array.BYTES_PER_ELEMENT; \
             let ab = new ArrayBuffer(b * 4, { maxByteLength: b * 5 }); \
             let view = new BigInt64Array(ab, b, 2); \
             ab.resize(b * 3 - 1); \
             let caught = false; \
             try { view.values(); } catch (e) { caught = e instanceof TypeError; } \
             caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn for_each_some_every_find() {
    assert_eq!(
        eval("let s = 0; new Uint8Array([1, 2, 3]).forEach(x => { s += x; }); s;"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("new Uint8Array([1, 2, 3]).some(x => x > 2);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("new Uint8Array([1, 2, 3]).every(x => x > 0);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("new Uint8Array([1, 2, 3]).find(x => x > 1);"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("new Uint8Array([1, 2, 3]).findLastIndex(x => x < 3);"),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn map_filter_slice_build_same_kind() {
    assert_eq!(
        eval(
            "let r = new Int16Array([1, 2, 3]).map(x => x * 2); r.constructor === Int16Array ? r.join(',') : 'wrong';"
        ),
        Ok(Value::String("2,4,6".to_owned()))
    );
    assert_eq!(
        eval(
            "let r = new Uint8Array([1, 2, 3, 4]).filter(x => x % 2 === 0); (r instanceof Uint8Array) + ':' + r.join(',');"
        ),
        Ok(Value::String("true:2,4".to_owned()))
    );
    assert_eq!(
        eval("new Uint8Array([1, 2, 3, 4]).slice(1, 3).join(',');"),
        Ok(Value::String("2,3".to_owned()))
    );
    // map applies per-type conversion to the callback result.
    assert_eq!(
        eval("new Uint8Array([1]).map(() => 257).join(',');"),
        Ok(Value::String("1".to_owned()))
    );
}

#[test]
fn reduce_and_reduce_right() {
    assert_eq!(
        eval("new Uint8Array([1, 2, 3]).reduce((a, x) => a + x, 100);"),
        Ok(Value::Number(106.0))
    );
    assert_eq!(
        eval("new Uint8Array([1, 2, 3]).reduceRight((a, x) => a + '' + x, '');"),
        Ok(Value::String("321".to_owned()))
    );
    assert!(eval("new Uint8Array([]).reduce((a, x) => a + x);").is_err());
}

#[test]
fn subarray_creates_shared_buffer_view() {
    assert_eq!(
        eval(
            "let base = new Uint8Array([1, 2, 3, 4]); \
             let view = base.subarray(1, 3); \
             view[0] = 20; \
             view.join(',') + '|' + base.join(',');"
        ),
        Ok(Value::String("20,3|1,20,3,4".to_owned()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let fixed = new Uint8Array(b, 0, 4); fixed.set([0, 2, 4, 6]); \
             b.resize(2); \
             let begin = { valueOf() { b.resize(4); return 0; } }; \
             let result = fixed.subarray(begin, 1); \
             result.length + ':' + result.join(',');"
        ),
        Ok(Value::String("0:".to_owned()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let fixed = new Uint8Array(b, 0, 4); fixed.set([0, 2, 4, 6]); \
             let begin = { valueOf() { b.resize(2); return 0; } }; \
             let smaller = fixed.subarray(begin, 1).join(','); \
             b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             fixed = new Uint8Array(b, 0, 4); fixed.set([0, 2, 4, 6]); \
             let threw = false; \
             try { fixed.subarray(0, { valueOf() { b.resize(2); return 3; } }); } \
             catch (e) { threw = e instanceof RangeError; } \
             smaller + ':' + threw;"
        ),
        Ok(Value::String("0:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let offset = new Uint8Array(b, 2, 2); \
             b.resize(1); \
             let threw = false; \
             try { offset.subarray(0); } catch (e) { threw = e instanceof RangeError; } \
             String(threw);"
        ),
        Ok(Value::String("true".to_owned()))
    );
}

// --- Prototype methods: write / order family (batch 2) -----------------------

#[test]
fn fill_writes_and_refreshes_reads() {
    // fill applies per-type conversion and keeps materialized index reads in
    // sync with the backing buffer.
    assert_eq!(
        eval(
            "let a = new Uint8Array(4); a.fill(257, 1, 3); a.join(',') + '|' + a[1] + ':' + a[3];"
        ),
        Ok(Value::String("0,1,1,0|1:0".to_owned()))
    );
}

#[test]
fn set_from_array_like_and_typed_array() {
    assert_eq!(
        eval("let a = new Uint8Array([0, 0, 0, 0]); a.set([10, 20], 1); a.join(',');"),
        Ok(Value::String("0,10,20,0".to_owned()))
    );
    assert_eq!(
        eval("let a = new Int16Array(3); a.set(new Uint8Array([5, 6])); a.join(',');"),
        Ok(Value::String("5,6,0".to_owned()))
    );
    // Out-of-range source throws RangeError.
    assert!(eval("new Uint8Array(2).set([1, 2, 3]);").is_err());
    // Negative offset throws RangeError.
    assert!(eval("new Uint8Array(4).set([1], -1);").is_err());
    // Mixing BigInt and Number typed arrays throws.
    assert!(eval("new BigInt64Array(2).set(new Uint8Array([1, 2]));").is_err());
}

#[test]
fn copy_within_handles_overlap() {
    assert_eq!(
        eval("let a = new Uint8Array([1, 2, 3, 4, 5]); a.copyWithin(0, 3); a.join(',');"),
        Ok(Value::String("4,5,3,4,5".to_owned()))
    );
    assert_eq!(
        eval("let a = new Uint8Array([1, 2, 3, 4, 5]); a.copyWithin(1, 0, 2); a.join(',');"),
        Ok(Value::String("1,1,2,4,5".to_owned()))
    );
}

#[test]
fn reverse_in_place_and_to_reversed_copies() {
    assert_eq!(
        eval(
            "let a = new Int8Array([1, 2, 3]); let r = a.reverse(); (r === a) + ':' + a.join(',');"
        ),
        Ok(Value::String("true:3,2,1".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([1, 2, 3]); let r = a.toReversed(); a.join(',') + '|' + r.join(',');"
        ),
        Ok(Value::String("1,2,3|3,2,1".to_owned()))
    );
}

#[test]
fn sort_default_is_numeric_and_stable() {
    // Default ordering is numeric, not the string ordering used by Array.
    assert_eq!(
        eval("new Uint8Array([3, 20, 100, 1]).sort().join(',');"),
        Ok(Value::String("1,3,20,100".to_owned()))
    );
    // NaN sorts last, -0 before +0.
    assert_eq!(
        eval(
            "[...new Float64Array([NaN, 1, -0, 0, -1]).sort()].map(x => Object.is(x, -0) ? 'n0' : x).join(',');"
        ),
        Ok(Value::String("-1,n0,0,1,NaN".to_owned()))
    );
    // Comparator overrides ordering.
    assert_eq!(
        eval("new Uint8Array([1, 2, 3]).sort((a, b) => b - a).join(',');"),
        Ok(Value::String("3,2,1".to_owned()))
    );
}

#[test]
fn to_sorted_copies_and_with_replaces() {
    assert_eq!(
        eval(
            "let a = new Int16Array([3, 1, 2]); let r = a.toSorted(); a.join(',') + '|' + r.join(',');"
        ),
        Ok(Value::String("3,1,2|1,2,3".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([1, 2, 3]); let r = a.with(1, 99); a.join(',') + '|' + r.join(',');"
        ),
        Ok(Value::String("1,2,3|1,99,3".to_owned()))
    );
    // Out-of-range index throws RangeError.
    assert!(eval("new Uint8Array(2).with(5, 1);").is_err());
}

#[test]
fn bigint_fill_rejects_number() {
    assert!(eval("new BigInt64Array(2).fill(5);").is_err());
    assert_eq!(
        eval("new BigInt64Array(2).fill(5n).join(',');"),
        Ok(Value::String("5,5".to_owned()))
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
        Ok(Value::String(":true".to_owned()))
    );
}

#[test]
fn uint8_array_set_from_hex_decodes_and_reports_progress() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([0, 0, 0, 0]); \
             let r = a.setFromHex('0aFf10'); \
             a.join(',') + '|' + r.read + ':' + r.written;"
        ),
        Ok(Value::String("10,255,16,0|6:3".to_owned()))
    );
    assert_eq!(
        eval(
            "let base = new Uint8Array([1, 2, 3, 4]); \
             let a = base.subarray(1, 3); \
             let r = a.setFromHex('aabbcc'); \
             a.join(',') + '|' + base.join(',') + '|' + r.read + ':' + r.written;"
        ),
        Ok(Value::String("170,187|1,170,187,4|4:2".to_owned()))
    );
}

#[test]
fn uint8_array_set_from_hex_surface_and_receiver_checks() {
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Uint8Array.prototype, 'setFromHex'); \
             d.writable + ':' + d.enumerable + ':' + d.configurable + ':' \
             + Uint8Array.prototype.setFromHex.name + ':' \
             + Uint8Array.prototype.setFromHex.length;"
        ),
        Ok(Value::String("true:false:true:setFromHex:1".to_owned()))
    );
    assert!(eval("new Uint8Array.prototype.setFromHex();").is_err());
    assert!(eval("Uint8Array.prototype.setFromHex.call(new Int8Array(1), '00');").is_err());
    assert!(eval("Uint8Array.prototype.setFromHex.call({}, '00');").is_err());
}

#[test]
fn uint8_array_set_from_hex_errors_preserve_specified_writes() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([1, 2]); \
             try { a.setFromHex('aaa'); } catch (e) { (e instanceof SyntaxError) + ':' + a.join(','); }"
        ),
        Ok(Value::String("true:1,2".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([1, 2]); \
             try { a.setFromHex('aaa '); } catch (e) { (e instanceof SyntaxError) + ':' + a.join(','); }"
        ),
        Ok(Value::String("true:170,2".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([1]); \
             let arg = { toString() { a[0] = 99; return '00'; } }; \
             try { a.setFromHex(arg); } catch (e) { (e instanceof TypeError) + ':' + a[0]; }"
        ),
        Ok(Value::String("true:1".to_owned()))
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
        Ok(Value::String(":true:4".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = ''; \
             let a = new Uint8Array(new ArrayBuffer(0).transferToImmutable()); \
             try { a.sort(() => { calls += 'compare'; return 0; }); } \
             catch (e) { calls + ':' + (e instanceof TypeError) + ':' + a.length; }"
        ),
        Ok(Value::String(":true:0".to_owned()))
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
        Ok(Value::String("7:1:3:7".to_owned()))
    );
    // A reversed view reads back consistently through materialized properties.
    assert_eq!(
        eval(
            "let a = new Int32Array(100); for (let i = 0; i < 100; i++) a.fill(0); \
             let b = a.map((_, i) => i); b.reverse(); b[0] + ':' + b[99];"
        ),
        Ok(Value::String("99:0".to_owned()))
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
        Ok(Value::String("0=10,1=20,2=30".to_owned()))
    );
    // Callback-driven reads must observe writes made by earlier callbacks.
    assert_eq!(
        eval(
            "let a = new Int8Array([42, 43, 44]); let seen = []; \
             a.forEach((v, i) => { seen.push(v); if (i < a.length - 1) a[i + 1] = 42; }); \
             seen.join(',');"
        ),
        Ok(Value::String("42,42,42".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new BigInt64Array([42n, 43n, 44n]); let seen = []; \
             a.forEach((v, i) => { seen.push(String(v)); if (i < a.length - 1) a[i + 1] = 42n; }); \
             seen.join(',');"
        ),
        Ok(Value::String("42,42,42".to_owned()))
    );
}
