use std::collections::HashMap;

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
    env: &mut HashMap<String, Value>,
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
    _env: &mut HashMap<String, Value>,
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

/// Writes an already-coerced `value` into element `index` of `object`,
/// persisting both the backing buffer and the materialized own property so
/// ordinary `array[i]` reads stay consistent. Coercion must happen first via
/// [`coerce_element`]. Used by the write/order-family methods.
pub(crate) fn set_view_element(object: &ObjectRef, index: usize, value: Value) {
    let native = typed_array_kind(object);
    if let Some(buffer) = super::typed_array_buffer(object) {
        if !array_buffer::is_detached(&buffer) {
            let mut bytes = array_buffer::array_buffer_bytes(&buffer);
            let element = bytes_per_element(native);
            let byte_index = typed_array_byte_offset(object) + index * element;
            write_element(native, &mut bytes, byte_index, &value);
            array_buffer::set_array_buffer_bytes(&buffer, bytes);
        }
    }
    object.define_property(index.to_string(), Property::data(value, true, true, false));
}
