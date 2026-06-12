use crate::CallEnv;

use num_bigint::BigInt;

use crate::{
    NativeFunction, ObjectRef, Property, RuntimeError, Value, array_buffer, to_number_with_env,
};

use super::{
    bytes_per_element, clamp_uint8, is_big_int_kind, modulo_integer, signed_integer,
    typed_array_byte_offset, typed_array_kind,
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
    Ok(Value::Number(value))
}

fn coerce_big_int_element(
    native: NativeFunction,
    value: Value,
    _env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    // ToBigInt: only BigInt and boolean coerce; numbers and the rest throw.
    let big = match value {
        Value::BigInt(value) => value,
        Value::Boolean(flag) => BigInt::from(u8::from(flag)),
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: cannot convert value to a BigInt typed array element"
                    .to_owned(),
            });
        }
    };
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

/// Reads `length` elements of `native` starting at byte `offset`.
pub(crate) fn read_elements(
    native: NativeFunction,
    bytes: &[u8],
    offset: usize,
    length: usize,
) -> Vec<Value> {
    let element = bytes_per_element(native);
    (0..length)
        .map(|index| read_element(native, bytes, offset + index * element))
        .collect()
}

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
    let encoded = encode_element(native, value);
    bytes[byte_index..byte_index + element].copy_from_slice(&encoded);
}

fn encode_element(native: NativeFunction, value: &Value) -> Vec<u8> {
    match native {
        NativeFunction::Uint8Array | NativeFunction::Uint8ClampedArray => {
            vec![number_of(value) as u8]
        }
        NativeFunction::Int8Array => vec![(number_of(value) as i64 as i8) as u8],
        NativeFunction::Uint16Array => (number_of(value) as i64 as u16).to_le_bytes().to_vec(),
        NativeFunction::Int16Array => (number_of(value) as i64 as i16).to_le_bytes().to_vec(),
        NativeFunction::Uint32Array => (number_of(value) as i64 as u32).to_le_bytes().to_vec(),
        NativeFunction::Int32Array => (number_of(value) as i64 as i32).to_le_bytes().to_vec(),
        NativeFunction::Float32Array => (number_of(value) as f32).to_le_bytes().to_vec(),
        NativeFunction::Float64Array => number_of(value).to_le_bytes().to_vec(),
        NativeFunction::BigInt64Array => big_int_of(value).to_le_bytes().to_vec(),
        NativeFunction::BigUint64Array => (big_int_of(value) as u64).to_le_bytes().to_vec(),
        _ => Vec::new(),
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

// --- view-level element access ----------------------------------------------

/// Reads element `index` of a branded typed-array view from its backing buffer.
/// Returns the neutral element if the buffer is detached or out of range.
pub(crate) fn get_view_element(object: &ObjectRef, index: usize) -> Value {
    let native = typed_array_kind(object);
    let Some(buffer) = super::typed_array_buffer(object) else {
        return zero_value(native);
    };
    if array_buffer::is_detached(&buffer) {
        return zero_value(native);
    }
    let bytes = array_buffer::array_buffer_bytes(&buffer);
    let element = bytes_per_element(native);
    let byte_index = typed_array_byte_offset(object) + index * element;
    read_element(native, &bytes, byte_index)
}

/// A one-shot decoded view of a typed array's backing bytes, used to read many
/// elements during a callback-driven iteration (forEach/map/reduce/...) without
/// re-decoding the byte string on every access. The buffer handle is retained
/// so each read can honor a detachment performed by a user callback: once the
/// buffer is detached, reads return the neutral element, matching the
/// element-at-a-time [`get_view_element`] behavior.
pub(crate) struct ViewSnapshot {
    native: NativeFunction,
    buffer: Option<ObjectRef>,
    bytes: Vec<u8>,
    base: usize,
    element: usize,
}

impl ViewSnapshot {
    /// Decodes the current backing bytes of `object` once.
    pub(crate) fn capture(object: &ObjectRef) -> Self {
        let native = typed_array_kind(object);
        let buffer =
            super::typed_array_buffer(object).filter(|buffer| !array_buffer::is_detached(buffer));
        let bytes = buffer
            .as_ref()
            .map(array_buffer::array_buffer_bytes)
            .unwrap_or_default();
        ViewSnapshot {
            native,
            buffer,
            bytes,
            base: typed_array_byte_offset(object),
            element: bytes_per_element(native),
        }
    }

    /// Reads element `index` from the captured bytes, or the neutral element if
    /// the buffer has since been detached.
    pub(crate) fn get(&self, index: usize) -> Value {
        match &self.buffer {
            Some(buffer) if !array_buffer::is_detached(buffer) => {
                read_element(self.native, &self.bytes, self.base + index * self.element)
            }
            _ => zero_value(self.native),
        }
    }
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
    let bytes = array_buffer::array_buffer_bytes(&buffer);
    let element = bytes_per_element(native);
    let base = typed_array_byte_offset(object);
    (0..count)
        .map(|offset| read_element(native, &bytes, base + (start + offset) * element))
        .collect()
}

/// Writes the already-coerced `values` into the contiguous element range
/// starting at `start`, persisting both the backing buffer and the materialized
/// own properties so ordinary `array[i]` reads stay consistent. Coercion must
/// happen first via [`coerce_element`]. Used by the write/order-family methods.
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

    let buffer =
        super::typed_array_buffer(object).filter(|buffer| !array_buffer::is_detached(buffer));

    match buffer {
        Some(buffer) => {
            let mut bytes = array_buffer::array_buffer_bytes(&buffer);
            for (offset, value) in values.into_iter().enumerate() {
                let index = start + offset;
                write_element(native, &mut bytes, base + index * element, &value);
                object.define_property(index.to_string(), Property::data(value, true, true, false));
            }
            array_buffer::set_array_buffer_bytes(&buffer, bytes);
        }
        None => {
            // Detached or buffer-less: keep the materialized properties in sync
            // even though there is no backing storage to write through.
            for (offset, value) in values.into_iter().enumerate() {
                let index = start + offset;
                object.define_property(index.to_string(), Property::data(value, true, true, false));
            }
        }
    }
}
