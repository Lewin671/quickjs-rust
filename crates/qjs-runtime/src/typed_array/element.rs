use crate::CallEnv;

use num_bigint::BigInt;

use crate::{
    NativeFunction, ObjectRef, RuntimeError, Value, array_buffer, bigint,
    object::PropertyDescriptor, to_number_with_env,
};

use super::{
    bytes_per_element, clamp_uint8, is_big_int_kind, modulo_integer, signed_integer,
    typed_array_byte_offset, typed_array_kind, typed_array_length,
    typed_array_length_for_buffer_byte_length,
};

/// Coerces an arbitrary value to the canonical element value for `native`,
/// applying the per-type numeric conversion (wrapping for integers, clamping
/// for `Uint8Clamped`, BigInt wrapping for the 64-bit kinds). The stored value
/// is always a `Number` (or `BigInt` for BigInt arrays).
pub(crate) fn coerce_element(
    native: NativeFunction,
    value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if is_big_int_kind(native) {
        return coerce_big_int_element(native, value, env);
    }

    let number = to_number_with_env(value, env)?;
    Ok(coerce_number_element(native, number))
}

fn coerce_number_element(native: NativeFunction, number: f64) -> Value {
    let value = match native {
        NativeFunction::Uint8Array => modulo_integer(number, 256.0),
        NativeFunction::Int8Array => signed_integer(number, 8),
        NativeFunction::Uint8ClampedArray => clamp_uint8(number),
        NativeFunction::Uint16Array => modulo_integer(number, 65_536.0),
        NativeFunction::Int16Array => signed_integer(number, 16),
        NativeFunction::Uint32Array => modulo_integer(number, 4_294_967_296.0),
        NativeFunction::Int32Array => signed_integer(number, 32),
        NativeFunction::Float32Array => f32_round(number),
        NativeFunction::Float64Array => number,
        _ => unreachable!("non-bigint typed array native expected"),
    };
    Value::Number(value)
}

fn coerce_big_int_element(
    native: NativeFunction,
    value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let big = bigint::to_bigint(value, env)?;
    Ok(Value::BigInt(wrap_big_int(native, big)))
}

fn wrap_big_int(native: NativeFunction, value: BigInt) -> BigInt {
    let modulo = BigInt::from(1u64) << 64;
    let mut wrapped = ((value % &modulo) + &modulo) % &modulo;
    if matches!(native, NativeFunction::BigInt64Array) {
        let sign = BigInt::from(1u64) << 63;
        if wrapped >= sign {
            wrapped -= &modulo;
        }
    }
    wrapped
}

/// Rounds a number to `f32` precision then back to `f64`, matching the storage
/// semantics of `Float32Array`.
fn f32_round(number: f64) -> f64 {
    f64::from(number as f32)
}

/// The neutral element for `native` (zero, BigInt zero for the 64-bit kinds).
pub(crate) fn zero_value(native: NativeFunction) -> Value {
    if is_big_int_kind(native) {
        Value::BigInt(BigInt::from(0))
    } else {
        Value::Number(0.0)
    }
}

// --- byte <-> element encoding ----------------------------------------------

pub(crate) fn read_element(native: NativeFunction, bytes: &[u8], byte_index: usize) -> Value {
    let element = bytes_per_element(native);
    let slice = bytes.get(byte_index..byte_index + element);
    let Some(slice) = slice else {
        return zero_value(native);
    };
    match native {
        NativeFunction::Uint8Array | NativeFunction::Uint8ClampedArray => {
            Value::Number(slice[0] as f64)
        }
        NativeFunction::Int8Array => Value::Number(slice[0] as i8 as f64),
        NativeFunction::Uint16Array => {
            Value::Number(u16::from_le_bytes([slice[0], slice[1]]) as f64)
        }
        NativeFunction::Int16Array => {
            Value::Number(i16::from_le_bytes([slice[0], slice[1]]) as f64)
        }
        NativeFunction::Uint32Array => {
            Value::Number(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64)
        }
        NativeFunction::Int32Array => {
            Value::Number(i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64)
        }
        NativeFunction::Float32Array => Value::Number(f64::from(f32::from_le_bytes([
            slice[0], slice[1], slice[2], slice[3],
        ]))),
        NativeFunction::Float64Array => {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(slice);
            Value::Number(f64::from_le_bytes(buf))
        }
        NativeFunction::BigInt64Array => {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(slice);
            Value::BigInt(BigInt::from(i64::from_le_bytes(buf)))
        }
        NativeFunction::BigUint64Array => {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(slice);
            Value::BigInt(BigInt::from(u64::from_le_bytes(buf)))
        }
        _ => zero_value(native),
    }
}

pub(crate) fn write_element(
    native: NativeFunction,
    bytes: &mut [u8],
    byte_index: usize,
    value: &Value,
) {
    let element = bytes_per_element(native);
    if byte_index + element > bytes.len() {
        return;
    }
    match native {
        NativeFunction::Uint8Array | NativeFunction::Uint8ClampedArray => {
            bytes[byte_index] = number_of(value) as u8;
        }
        NativeFunction::Int8Array => {
            bytes[byte_index] = (number_of(value) as i64 as i8) as u8;
        }
        NativeFunction::Uint16Array => bytes[byte_index..byte_index + element]
            .copy_from_slice(&(number_of(value) as i64 as u16).to_le_bytes()),
        NativeFunction::Int16Array => bytes[byte_index..byte_index + element]
            .copy_from_slice(&(number_of(value) as i64 as i16).to_le_bytes()),
        NativeFunction::Uint32Array => bytes[byte_index..byte_index + element]
            .copy_from_slice(&(number_of(value) as i64 as u32).to_le_bytes()),
        NativeFunction::Int32Array => bytes[byte_index..byte_index + element]
            .copy_from_slice(&(number_of(value) as i64 as i32).to_le_bytes()),
        NativeFunction::Float32Array => bytes[byte_index..byte_index + element]
            .copy_from_slice(&(number_of(value) as f32).to_le_bytes()),
        NativeFunction::Float64Array => {
            bytes[byte_index..byte_index + element].copy_from_slice(&number_of(value).to_le_bytes())
        }
        NativeFunction::BigInt64Array => bytes[byte_index..byte_index + element]
            .copy_from_slice(&big_int_of(value).to_le_bytes()),
        NativeFunction::BigUint64Array => bytes[byte_index..byte_index + element]
            .copy_from_slice(&(big_int_of(value) as u64).to_le_bytes()),
        _ => {}
    }
}

fn number_of(value: &Value) -> f64 {
    match value {
        Value::Number(number) => *number,
        _ => 0.0,
    }
}

fn big_int_of(value: &Value) -> i64 {
    use num_traits::ToPrimitive;
    match value {
        Value::BigInt(big) => {
            // Take the low 64 bits.
            let modulo = BigInt::from(1u128 << 64);
            let wrapped = ((big % &modulo) + &modulo) % &modulo;
            wrapped.to_u64().map(|value| value as i64).unwrap_or(0)
        }
        _ => 0,
    }
}

/// Whether `key` is a CanonicalNumericIndexString: the string form of a Number
/// such that `String(ToNumber(key)) === key`. These are the keys that a typed
/// array treats as integer-indexed exotic slots (so writes to them never create
/// ordinary properties), e.g. `"0"`, `"-0"`, `"1.5"`, `"Infinity"`, `"NaN"`.
/// Returns the numeric value when `key` is canonical.
pub(crate) fn canonical_numeric_index(key: &str) -> Option<f64> {
    if key == "-0" {
        return Some(-0.0);
    }
    let number: f64 = key.parse().ok()?;
    // Reparse round-trips through Rust formatting which differs from JS for some
    // inputs; require the JS-style string form to match exactly.
    if crate::number::number_to_js_string(number) == key {
        Some(number)
    } else {
        None
    }
}

/// Result of attempting a typed-array integer-indexed write
/// (IntegerIndexedElementSet, ES2024 10.4.5.16).
pub(crate) enum IndexedWrite {
    /// `key` was a canonical numeric index; the write was fully handled here
    /// (value coerced and, when in range with an attached buffer, stored). The
    /// ordinary property path must not run.
    Handled,
    /// `key` is not a canonical numeric index; the caller should fall back to
    /// the ordinary property-set path.
    NotIndexed,
}

/// Result of attempting a receiver-side typed-array indexed data definition.
pub(crate) enum IndexedDefine {
    /// `key` was a canonical numeric index and the define operation succeeded.
    Defined,
    /// `key` was a canonical numeric index but cannot be defined on this view.
    Rejected,
    /// `key` is not a canonical numeric index; use ordinary property definition.
    NotIndexed,
}

/// Result of attempting a typed-array integer-indexed delete operation.
pub(crate) enum IndexedDelete {
    /// `key` was a canonical numeric index and the delete result was handled.
    Handled(bool),
    /// `key` is not a canonical numeric index.
    NotIndexed,
}

/// Result of resolving a typed-array integer-indexed read/existence query.
pub(crate) enum IndexedRead {
    /// `key` was a canonical numeric index and the element exists.
    Present(Box<Value>),
    /// `key` was a canonical numeric index but not a valid element index.
    Missing,
    /// `key` is not a canonical numeric index; use ordinary property lookup.
    NotIndexed,
}

pub(crate) fn indexed_element_value(object: &ObjectRef, key: &str) -> IndexedRead {
    let Some(number) = canonical_numeric_index(key) else {
        return IndexedRead::NotIndexed;
    };
    let Some(index) = valid_integer_index(object, number) else {
        return IndexedRead::Missing;
    };
    IndexedRead::Present(Box::new(get_view_element(object, index)))
}

/// IntegerIndexedElementGet by a `usize` index, skipping the string round-trip
/// of `canonical_numeric_index`. A typed array's integer index is owned by the
/// exotic `[[Get]]`, so an out-of-range or detached read yields `undefined`
/// (never a prototype lookup). Used by the VM's integer-index fast path.
pub(crate) fn integer_indexed_value(object: &ObjectRef, index: usize) -> Value {
    match valid_integer_index(object, index as f64) {
        Some(index) => get_view_element(object, index),
        None => Value::Undefined,
    }
}

/// Performs the integer-indexed write for `key = value` on a branded typed
/// array. Coercion of `value` always runs first (its observable side effects
/// must happen even for out-of-range or detached writes). When `key` names a
/// canonical numeric index, the write is handled here: stored into the backing
/// buffer and materialized property when the index is in range and the buffer
/// is attached, and silently dropped otherwise.
pub(crate) fn set_indexed_element(
    object: &ObjectRef,
    key: &str,
    value: Value,
    env: &mut CallEnv,
) -> Result<IndexedWrite, RuntimeError> {
    let Some(number) = canonical_numeric_index(key) else {
        return Ok(IndexedWrite::NotIndexed);
    };

    let native = typed_array_kind(object);
    // ToNumber/ToBigInt side effects run regardless of whether the slot is in
    // range; a coercion that throws propagates.
    let coerced = coerce_element(native, value, env)?;

    // Out-of-range, fractional, negative-zero, or non-integer indices, and
    // writes through a detached buffer, are all dropped without creating a
    // property — but only after the coercion above has run.
    if super::typed_array_buffer_detached(object) {
        return Ok(IndexedWrite::Handled);
    }
    let Some(index) = valid_integer_index(object, number) else {
        return Ok(IndexedWrite::Handled);
    };

    set_view_element(object, index, coerced);
    Ok(IndexedWrite::Handled)
}

/// IntegerIndexedElementSet by a `usize` index, skipping the string round-trip
/// and canonical numeric index parse when the VM has already classified a
/// numeric property key. Coercion still runs before detached/out-of-bounds
/// checks so observable `valueOf` effects match the generic path.
pub(crate) fn set_integer_indexed_element(
    object: &ObjectRef,
    index: usize,
    value: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let native = typed_array_kind(object);
    let coerced = coerce_element(native, value, env)?;
    if super::typed_array_buffer_detached(object) || index >= typed_array_length(object) {
        return Ok(());
    }
    set_view_element(object, index, coerced);
    Ok(())
}

/// Attempts IntegerIndexedElementSet for primitive values that need no
/// environment-backed coercion. Returning `false` means the caller must use the
/// generic path so objects, strings, booleans, and cross-kind BigInt/Number
/// errors keep their observable conversion behavior.
pub(crate) fn try_set_integer_indexed_primitive_element(
    object: &ObjectRef,
    index: usize,
    value: &Value,
) -> bool {
    let native = typed_array_kind(object);
    let coerced = match (is_big_int_kind(native), value) {
        (false, Value::Number(number)) => coerce_number_element(native, *number),
        (true, Value::BigInt(big)) => Value::BigInt(wrap_big_int(native, big.clone())),
        _ => return false,
    };
    if super::typed_array_buffer_detached(object) || index >= typed_array_length(object) {
        return true;
    }
    set_view_element(object, index, coerced);
    true
}

/// Defines the value descriptor used by OrdinarySet when a typed array is the
/// receiver rather than the original target. This follows the typed-array
/// [[DefineOwnProperty]] path, so invalid indices reject without running value
/// coercion.
pub(crate) fn define_indexed_element_value(
    object: &ObjectRef,
    key: &str,
    value: Value,
    env: &mut CallEnv,
) -> Result<IndexedDefine, RuntimeError> {
    let Some(number) = canonical_numeric_index(key) else {
        return Ok(IndexedDefine::NotIndexed);
    };
    let Some(index) = valid_integer_index(object, number) else {
        return Ok(IndexedDefine::Rejected);
    };

    let native = typed_array_kind(object);
    let coerced = coerce_element(native, value, env)?;
    set_view_element(object, index, coerced);
    Ok(IndexedDefine::Defined)
}

/// Implements the integer-indexed branch of TypedArray [[DefineOwnProperty]].
pub(crate) fn define_indexed_property_descriptor(
    object: &ObjectRef,
    key: &str,
    descriptor: &PropertyDescriptor,
    env: &mut CallEnv,
) -> Result<IndexedDefine, RuntimeError> {
    let Some(number) = canonical_numeric_index(key) else {
        return Ok(IndexedDefine::NotIndexed);
    };
    if valid_integer_index(object, number).is_none() {
        return Ok(IndexedDefine::Rejected);
    }
    if descriptor.is_accessor_descriptor()
        || descriptor.configurable_field() == Some(false)
        || descriptor.enumerable_field() == Some(false)
        || descriptor.writable_field() == Some(false)
    {
        return Ok(IndexedDefine::Rejected);
    }
    if let Some(value) = descriptor.value_field() {
        let IndexedWrite::Handled = set_indexed_element(object, key, value.clone(), env)? else {
            unreachable!("canonical numeric index must be handled by typed-array setter");
        };
    }
    Ok(IndexedDefine::Defined)
}

/// Implements the integer-indexed branch of TypedArray [[Delete]].
pub(crate) fn delete_indexed_element(object: &ObjectRef, key: &str) -> IndexedDelete {
    let Some(number) = canonical_numeric_index(key) else {
        return IndexedDelete::NotIndexed;
    };
    IndexedDelete::Handled(valid_integer_index(object, number).is_none())
}

fn valid_integer_index(object: &ObjectRef, number: f64) -> Option<usize> {
    if super::typed_array_buffer_detached(object)
        || !number.is_finite()
        || number.fract() != 0.0
        || number.is_sign_negative()
    {
        return None;
    }
    let index = number as usize;
    (index < typed_array_length(object)).then_some(index)
}

// --- view-level element access ----------------------------------------------

/// Reads element `index` of a branded typed-array view from its backing buffer,
/// implementing IntegerIndexedElementGet: a detached buffer or an index outside
/// the view's current bounds (e.g. after a resizable buffer shrank mid-loop)
/// yields `undefined`, not the neutral element. Callers that need the neutral
/// element for a valid index never hit these branches: the indexed-read path
/// and `at` resolve the bounds first.
pub(crate) fn get_view_element(object: &ObjectRef, index: usize) -> Value {
    let native = typed_array_kind(object);
    let Some(buffer) = super::typed_array_buffer(object) else {
        return Value::Undefined;
    };
    if array_buffer::is_detached(&buffer) {
        return Value::Undefined;
    }
    let element = bytes_per_element(native);
    let byte_index = typed_array_byte_offset(object) + index * element;
    array_buffer::with_buffer_bytes(&buffer, |bytes| {
        if index >= typed_array_length_for_buffer_byte_length(object, bytes.len()) {
            return Value::Undefined;
        }
        read_element(native, bytes, byte_index)
    })
}

/// Reads `count` elements of a branded typed-array view starting at `start`,
/// decoding the backing-buffer bytes exactly once. Returns neutral elements for
/// a detached or buffer-less view. This is the bulk counterpart to
/// [`get_view_element`]: a `(0..length).map(get_view_element)` snapshot would
/// re-decode the whole byte string per element (O(n^2)); this stays O(n).
pub(crate) fn read_view_elements(object: &ObjectRef, start: usize, count: usize) -> Vec<Value> {
    let native = typed_array_kind(object);
    let buffer =
        super::typed_array_buffer(object).filter(|buffer| !array_buffer::is_detached(buffer));
    let Some(buffer) = buffer else {
        return std::iter::repeat_n(zero_value(native), count).collect();
    };
    let element = bytes_per_element(native);
    let base = typed_array_byte_offset(object);
    array_buffer::with_buffer_bytes(&buffer, |bytes| {
        let length = typed_array_length_for_buffer_byte_length(object, bytes.len());
        (0..count)
            .map(|offset| {
                let index = start + offset;
                if index < length {
                    read_element(native, bytes, base + index * element)
                } else {
                    zero_value(native)
                }
            })
            .collect()
    })
}

/// Writes the already-coerced `values` into the contiguous element range
/// starting at `start`, persisting the backing buffer. Coercion must happen
/// first via [`coerce_element`]. Used by the write/order-family methods.
///
/// The backing-buffer bytes are decoded once, mutated in place, and re-encoded
/// once. This keeps fill/set/copyWithin/sort/reverse at O(n) total instead of
/// O(n) per element (the byte buffer is a string slot, so a per-element
/// read-modify-write round trip would be O(n^2)).
pub(crate) fn set_view_elements<I>(object: &ObjectRef, start: usize, values: I)
where
    I: IntoIterator<Item = Value>,
{
    let native = typed_array_kind(object);
    let element = bytes_per_element(native);
    let base = typed_array_byte_offset(object);
    let length = typed_array_length(object);

    let buffer =
        super::typed_array_buffer(object).filter(|buffer| !array_buffer::is_detached(buffer));

    match buffer {
        Some(buffer) => {
            let mut bytes = array_buffer::buffer_bytes(&buffer);
            for (offset, value) in values.into_iter().enumerate() {
                let index = start + offset;
                if index >= length {
                    continue;
                }
                write_element(native, &mut bytes, base + index * element, &value);
            }
            array_buffer::set_buffer_bytes(&buffer, bytes);
        }
        None => {
            // Detached or buffer-less: consume the iterator so callers keep the
            // same eager behavior, but do not create ordinary index properties.
            for _ in values {}
        }
    }
}

fn set_view_element(object: &ObjectRef, index: usize, value: Value) {
    let native = typed_array_kind(object);
    let element = bytes_per_element(native);
    let byte_index = typed_array_byte_offset(object) + index * element;
    let length = typed_array_length(object);
    if index >= length {
        return;
    }

    let buffer =
        super::typed_array_buffer(object).filter(|buffer| !array_buffer::is_detached(buffer));
    if let Some(buffer) = buffer {
        let _ = array_buffer::mutate_array_buffer_bytes(&buffer, |bytes| {
            write_element(native, bytes, byte_index, &value);
        });
    }
}
