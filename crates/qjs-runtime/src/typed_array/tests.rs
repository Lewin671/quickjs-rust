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
fn construct_from_buffer_validates_alignment_and_bounds() {
    // Misaligned offset.
    assert!(eval("new Uint32Array(new ArrayBuffer(8), 2);").is_err());
    // Length out of range.
    assert!(eval("new Uint8Array(new ArrayBuffer(4), 0, 8);").is_err());
    // Buffer not aligned to element size with implicit length.
    assert!(eval("new Uint32Array(new ArrayBuffer(6));").is_err());
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
    assert!(eval("new BigInt64Array([1]);").is_err());
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
fn subarray_copies_range() {
    assert_eq!(
        eval("new Uint8Array([1, 2, 3, 4]).subarray(1, 3).join(',');"),
        Ok(Value::String("2,3".to_owned()))
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
}
