//! `%TypedArray.prototype%` iteration and read-family tests (at, includes,
//! join/toString/toLocaleString, iterators, map/filter/slice/subarray species,
//! reduce). Split from `tests.rs` to keep first-party files reviewable.

use crate::{Value, eval};

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
fn last_index_of_coerces_from_index_and_rechecks_view_length() {
    assert_eq!(
        eval("let a = new Uint8Array([42, 43]); a.lastIndexOf(43, undefined);"),
        Ok(Value::Number(-1.0))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([0]); \
             a.lastIndexOf(0, { valueOf() { __quickjsRustDetachArrayBuffer(a.buffer); return 0; } });"
        ),
        Ok(Value::Number(-1.0))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let a = new Uint8Array(b); \
             a[0] = 0; a[1] = 1; a[2] = 2; a[3] = 3; \
             a.lastIndexOf(2, { valueOf() { b.resize(2); return 2; } });"
        ),
        Ok(Value::Number(-1.0))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let a = new Uint8Array(b); \
             a[0] = 1; a[1] = 1; a[2] = 1; a[3] = 1; \
             a.lastIndexOf(0, { valueOf() { b.resize(6); return -1; } });"
        ),
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
fn to_locale_string_invokes_each_element() {
    // toLocaleString Invokes the element's own toLocaleString (resolved through
    // Number.prototype, so an override is honored) and ToStrings the result.
    assert_eq!(
        eval(
            "Number.prototype.toLocaleString = function() { return 'n' + this; }; \
             new Uint8Array([1, 2, 3]).toLocaleString();"
        ),
        Ok(Value::String("n1,n2,n3".to_owned()))
    );
    // The Invoke result is ToString-coerced: an object result runs toString.
    assert_eq!(
        eval(
            "Number.prototype.toLocaleString = function() { \
                 return { toString: function() { return 'X'; }, \
                          valueOf: function() { throw new Error('no valueOf'); } }; \
             }; \
             new Uint8Array([4, 5]).toLocaleString();"
        ),
        Ok(Value::String("X,X".to_owned()))
    );
    // A throwing element toLocaleString propagates.
    assert!(
        eval(
            "Number.prototype.toLocaleString = function() { throw new TypeError('boom'); }; \
             new Uint8Array([1]).toLocaleString();"
        )
        .is_err()
    );
    // A throwing ToString of the Invoke result propagates.
    assert!(
        eval(
            "Number.prototype.toLocaleString = function() { \
                 return { toString: function() { throw new TypeError('boom'); } }; \
             }; \
             new Uint8Array([1]).toLocaleString();"
        )
        .is_err()
    );
    // If the view shrinks during iteration, out-of-bounds elements read as
    // undefined and contribute empty strings.
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let a = new Uint8Array(b); \
             let remaining = 2; \
             Number.prototype.toLocaleString = function() { \
                 remaining--; \
                 if (remaining === 0) { b.resize(2); } \
                 return '0'; \
             }; \
             a.toLocaleString();"
        ),
        Ok(Value::String("0,0,,".to_owned()))
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
fn slice_uses_species_constructor() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([40, 41, 42]); \
             let observed = ''; \
             a.constructor = {}; \
             a.constructor[Symbol.species] = function(count) { observed = String(count); return new Int16Array(count); }; \
             let r = a.slice(1); \
             observed + ':' + (r instanceof Int16Array) + ':' + r.join(',');"
        ),
        Ok(Value::String("2:true:41,42".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([1]); \
             let other = new Int8Array([5, 6]); \
             a.constructor = {}; \
             a.constructor[Symbol.species] = function() { return other; }; \
             let r = a.slice(0, 0); \
             (r === other) + ':' + r.join(',');"
        ),
        Ok(Value::String("true:5,6".to_owned()))
    );
}

#[test]
fn slice_rejects_invalid_species_result() {
    assert!(
        eval(
            "let a = new Uint8Array([1, 2]); \
             a.constructor = {}; \
             a.constructor[Symbol.species] = function() { return {}; }; \
             a.slice();"
        )
        .is_err()
    );
    assert!(
        eval(
            "let a = new Uint8Array([1, 2]); \
             a.constructor = {}; \
             a.constructor[Symbol.species] = 0; \
             a.slice();"
        )
        .is_err()
    );
}

#[test]
fn slice_rechecks_source_after_species_constructor() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([7]); \
             let calls = 0; \
             a.constructor = {}; \
             a.constructor[Symbol.species] = function(count) { calls++; __quickjsRustDetachArrayBuffer(a.buffer); return new Uint8Array(count); }; \
             let threw = false; \
             try { a.slice(); } catch (e) { threw = e instanceof TypeError; } \
             calls + ':' + threw;"
        ),
        Ok(Value::String("1:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([7]); \
             let calls = 0; \
             a.constructor = {}; \
             a.constructor[Symbol.species] = function(count) { calls++; __quickjsRustDetachArrayBuffer(a.buffer); return new Uint8Array(count); }; \
             let r = a.slice(0, 0); \
             calls + ':' + r.length;"
        ),
        Ok(Value::String("1:0".to_owned()))
    );
}

#[test]
fn map_filter_use_species_constructor() {
    // map allocates the result through @@species, called once with the source
    // length, and sets the per-type-coerced mapped values on the returned array.
    assert_eq!(
        eval(
            "let a = new Uint8Array([40, 41]); \
             let observed = ''; \
             a.constructor = {}; \
             a.constructor[Symbol.species] = function(count) { observed = String(count); return new Int16Array(count); }; \
             let r = a.map(x => x + 7); \
             observed + ':' + (r instanceof Int16Array) + ':' + r.join(',');"
        ),
        Ok(Value::String("2:true:47,48".to_owned()))
    );
    // filter calls @@species after every callback, with the captured count, and
    // a custom constructor result receives the kept values.
    assert_eq!(
        eval(
            "let a = new Uint8Array([1, 2, 3, 4]); \
             let observed = ''; \
             a.constructor = {}; \
             a.constructor[Symbol.species] = function(count) { observed = String(count); return new Int16Array(count); }; \
             let r = a.filter(x => x % 2 === 0); \
             observed + ':' + (r instanceof Int16Array) + ':' + r.join(',');"
        ),
        Ok(Value::String("2:true:2,4".to_owned()))
    );
    // A species constructor returning a different instance is used verbatim.
    assert_eq!(
        eval(
            "let a = new Int8Array([40]); \
             let other = new Int16Array([1, 0, 1]); \
             a.constructor = {}; \
             a.constructor[Symbol.species] = function() { return other; }; \
             let r = a.map(x => x + 7); \
             (r === other) + ':' + r.join(',');"
        ),
        Ok(Value::String("true:47,0,1".to_owned()))
    );
}

#[test]
fn map_filter_observe_species_ordering() {
    // filter fires every callback before reading @@species.
    assert_eq!(
        eval(
            "let a = new Uint8Array(3); \
             let calls = 0; \
             let before = false; \
             a.constructor = {}; \
             Object.defineProperty(a.constructor, Symbol.species, { get() { before = calls === 3; return Uint8Array; } }); \
             a.filter(() => { calls++; }); \
             calls + ':' + before;"
        ),
        Ok(Value::String("3:true".to_owned()))
    );
    // map does not cache source values: a callback mutation is visible later.
    assert_eq!(
        eval(
            "let a = new Uint8Array([42, 0, 0]); \
             let seen = []; \
             a.map(function(v, i) { if (i < 2) a[i + 1] = 42; seen.push(v); return v; }); \
             seen.join(',');"
        ),
        Ok(Value::String("42,42,42".to_owned()))
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
fn iteration_methods_read_values_live() {
    // find/findIndex/some/every/reduce read each element at call time, so a
    // callback that mutates a not-yet-visited index is observed, not a snapshot.
    assert_eq!(
        eval(
            "let a = new Uint8Array([1, 2, 3]); \
             let r = a.find(function(v, i) { if (i === 0) { a[2] = 7; } return v === 7; }); \
             String(r);"
        ),
        Ok(Value::String("7".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([0, 0, 0]); \
             let r = a.findIndex(function(v, i) { if (i === 0) { a[2] = 5; } return v === 5; }); \
             String(r);"
        ),
        Ok(Value::String("2".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array([1, 1, 1]); \
             let r = a.some(function(v, i) { if (i === 0) { a[2] = 9; } return v === 9; }); \
             String(r);"
        ),
        Ok(Value::String("true".to_owned()))
    );
    // findLast walks high-to-low; mutating a lower, not-yet-visited index is seen.
    assert_eq!(
        eval(
            "let a = new Uint8Array([0, 0, 3]); \
             let r = a.findLast(function(v, i) { if (i === 2) { a[0] = 8; } return v === 8; }); \
             String(r);"
        ),
        Ok(Value::String("8".to_owned()))
    );
    // reduce sees a value written by an earlier callback step.
    assert_eq!(
        eval(
            "let a = new Uint8Array([1, 0, 0]); \
             a.reduce(function(acc, v, i) { if (i === 0) { a[2] = 5; } return acc + v; }, 0);"
        ),
        Ok(Value::Number(6.0))
    );
}

#[test]
fn iteration_reads_undefined_past_shrunk_or_detached_bounds() {
    // IntegerIndexedElementGet returns undefined for an index outside the
    // view's current bounds: a callback that shrinks the backing resizable
    // buffer mid-loop makes later reads undefined, not the neutral element.
    assert_eq!(
        eval(
            "let rab = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let v = new Uint8Array(rab, 0, 4); v.set([0, 2, 4, 6]); \
             let out = []; let n = 0; \
             v.forEach(function(x) { out.push(x); if (++n === 2) { rab.resize(3); } }); \
             out.map(String).join(',');"
        ),
        Ok(Value::String("0,2,undefined,undefined".to_owned()))
    );
    // Detaching mid-iteration likewise reads undefined for the remaining slots.
    assert_eq!(
        eval(
            "let a = new Uint8Array([5, 6, 7, 8]); let seen = []; let n = 0; \
             a.forEach(function(x) { seen.push(x); if (++n === 2) { __quickjsRustDetachArrayBuffer(a.buffer); } }); \
             seen.map(String).join(',');"
        ),
        Ok(Value::String("5,6,undefined,undefined".to_owned()))
    );
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

#[test]
fn subarray_uses_species_constructor() {
    // subarray allocates through @@species with (buffer, beginByteOffset,
    // newLength); a custom constructor is invoked once and used verbatim.
    assert_eq!(
        eval(
            "let a = new Uint8Array([40, 41, 42]); \
             let args = ''; \
             a.constructor = {}; \
             a.constructor[Symbol.species] = function(buffer, offset, length) { \
                 args = (buffer instanceof ArrayBuffer) + ',' + offset + ',' + length; \
                 return new Uint8Array(buffer, offset, length); \
             }; \
             let r = a.subarray(1); \
             args + ':' + r.join(',');"
        ),
        Ok(Value::String("true,1,2:41,42".to_owned()))
    );
}

#[test]
fn subarray_on_detached_coerces_then_throws() {
    // A detached buffer yields srcLength 0 but the relative-index arguments are
    // still coerced (observable valueOf) before construction throws.
    assert_eq!(
        eval(
            "let a = new Int8Array(2); \
             let seen = ''; \
             __quickjsRustDetachArrayBuffer(a.buffer); \
             let begin = { valueOf() { seen += 'b'; return 0; } }; \
             let end = { valueOf() { seen += 'e'; return 2; } }; \
             let threw = false; \
             try { a.subarray(begin, end); } catch (e) { threw = e instanceof TypeError; } \
             seen + ':' + threw;"
        ),
        Ok(Value::String("be:true".to_owned()))
    );
}
