use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, array, array_buffer,
    function_prototype, property_value, to_number_with_env,
};

use super::{
    TYPED_ARRAY_BUFFER_PROPERTY, TYPED_ARRAY_BYTE_OFFSET_PROPERTY, TYPED_ARRAY_KIND_PROPERTY,
    TYPED_ARRAY_LENGTH_PROPERTY, bytes_per_element, coerce_element, is_big_int_kind,
    is_typed_array_object, to_typed_array_length, typed_array_kind, typed_array_length,
    typed_array_name,
};

pub(crate) fn native_typed_array(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    // %TypedArray% itself is abstract: never callable or constructable.
    if matches!(native, NativeFunction::TypedArray) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: %TypedArray% is not directly constructable".to_owned(),
        });
    }

    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: format!(
                "TypeError: Constructor {} requires 'new'",
                typed_array_name(native)
            ),
        });
    }

    let object = match this_value {
        Value::Object(object) => object,
        _ => ObjectRef::with_prototype(HashMap::new(), function_prototype(function)),
    };

    match argument_values.first().cloned() {
        None | Some(Value::Undefined) => initialize_from_length(&object, native, 0, env),
        Some(Value::Object(source)) if is_typed_array_object(&source) => {
            initialize_from_typed_array(&object, native, &source, env)
        }
        Some(Value::Object(source)) if array_buffer::is_array_buffer_object(&source) => {
            initialize_from_buffer(&object, native, source, argument_values, env)
        }
        Some(
            source @ (Value::Object(_)
            | Value::Array(_)
            | Value::Function(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Proxy(_)),
        ) => initialize_from_object(&object, native, source, env),
        Some(other) => {
            let length = to_typed_array_length(other, env)?;
            initialize_from_length(&object, native, length, env)
        }
    }?;

    Ok(Value::Object(object))
}

/// `new TA(length)` and `new TA()`: a fresh zero-filled buffer.
fn initialize_from_length(
    object: &ObjectRef,
    native: NativeFunction,
    length: usize,
    env: &HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let element = bytes_per_element(native);
    let byte_length = length
        .checked_mul(element)
        .ok_or_else(invalid_length_error)?;
    let buffer = array_buffer_for(byte_length, env);
    let zero = if is_big_int_kind(native) {
        Value::BigInt(num_bigint::BigInt::from(0))
    } else {
        Value::Number(0.0)
    };
    let values = std::iter::repeat_n(zero, length).collect();
    install_view(object, native, buffer, 0, length, values);
    Ok(())
}

/// `new TA(typedArray)`: element-by-element conversion into a new buffer.
fn initialize_from_typed_array(
    object: &ObjectRef,
    native: NativeFunction,
    source: &ObjectRef,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    if super::typed_array_buffer_detached(source) {
        return Err(array_buffer::detached_error());
    }
    let source_kind = typed_array_kind(source);
    // Cross-kind BigInt/Number conversion is forbidden by ToBigInt/ToNumber.
    if is_big_int_kind(native) != is_big_int_kind(source_kind) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: cannot mix BigInt and Number typed arrays".to_owned(),
        });
    }
    let length = typed_array_length(source);
    let mut values = Vec::with_capacity(length);
    for index in 0..length {
        let element = property_value(Value::Object(source.clone()), &index.to_string(), env)?;
        values.push(coerce_element(native, element, env)?);
    }
    let buffer = array_buffer_for(length * bytes_per_element(native), env);
    install_view(object, native, buffer, 0, length, values);
    Ok(())
}

/// `new TA(buffer [, byteOffset [, length]])`: a view over an existing buffer.
fn initialize_from_buffer(
    object: &ObjectRef,
    native: NativeFunction,
    buffer: ObjectRef,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let element = bytes_per_element(native);
    let offset = to_index(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if offset % element != 0 {
        return Err(range_error("start offset is not aligned to element size"));
    }

    let length_arg = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    let explicit_length = if matches!(length_arg, Value::Undefined) {
        None
    } else {
        Some(to_index(length_arg, env)?)
    };

    if array_buffer::is_detached(&buffer) {
        return Err(array_buffer::detached_error());
    }
    let buffer_byte_length = array_buffer::array_buffer_bytes(&buffer).len();

    let (length, byte_length) = match explicit_length {
        Some(length) => {
            let byte_length = length
                .checked_mul(element)
                .ok_or_else(invalid_length_error)?;
            if offset
                .checked_add(byte_length)
                .ok_or_else(invalid_length_error)?
                > buffer_byte_length
            {
                return Err(range_error("invalid typed array length"));
            }
            (length, byte_length)
        }
        None => {
            if buffer_byte_length % element != 0 {
                return Err(range_error("buffer length is not aligned to element size"));
            }
            if offset > buffer_byte_length {
                return Err(range_error("start offset is outside the buffer bounds"));
            }
            let byte_length = buffer_byte_length - offset;
            (byte_length / element, byte_length)
        }
    };

    let bytes = array_buffer::array_buffer_bytes(&buffer);
    let values = read_elements(native, &bytes, offset, length);
    let _ = byte_length;
    install_view(object, native, buffer, offset, length, values);
    Ok(())
}

/// `new TA(object)`: iterable (via `Symbol.iterator`) or array-like.
fn initialize_from_object(
    object: &ObjectRef,
    native: NativeFunction,
    source: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let iterator_method = match crate::symbol::iterator_symbol(env) {
        Some(symbol) => {
            crate::property_value_key(source.clone(), &crate::PropertyKey::Symbol(symbol), env)?
        }
        None => Value::Undefined,
    };

    let raw_values = if matches!(iterator_method, Value::Function(_)) {
        array::iterable_values_with_env(source, "TypedArray", env)?
    } else {
        array::array_like_values_with_env(source, "TypedArray", env)?
    };

    let mut values = Vec::with_capacity(raw_values.len());
    for value in raw_values {
        values.push(coerce_element(native, value, env)?);
    }
    let length = values.len();
    let buffer = array_buffer_for(length * bytes_per_element(native), env);
    install_view(object, native, buffer, 0, length, values);
    Ok(())
}

/// Allocates the backing buffer for a freshly created typed array, inheriting
/// `%ArrayBuffer.prototype%`.
fn array_buffer_for(byte_length: usize, env: &HashMap<String, Value>) -> ObjectRef {
    array_buffer::new_array_buffer(env, byte_length)
}

/// Writes the internal slots and materializes the indexed element properties so
/// ordinary `array[i]` reads resolve through the standard property path.
fn install_view(
    object: &ObjectRef,
    native: NativeFunction,
    buffer: ObjectRef,
    byte_offset: usize,
    length: usize,
    values: Vec<Value>,
) {
    let name = typed_array_name(native);
    object.set_to_string_tag(name);
    object.define_property(
        TYPED_ARRAY_KIND_PROPERTY.to_owned(),
        Property::non_enumerable(Value::String(name.to_owned())),
    );
    object.define_property(
        TYPED_ARRAY_BUFFER_PROPERTY.to_owned(),
        Property::non_enumerable(Value::Object(buffer.clone())),
    );
    object.define_property(
        TYPED_ARRAY_BYTE_OFFSET_PROPERTY.to_owned(),
        Property::non_enumerable(Value::Number(byte_offset as f64)),
    );
    object.define_property(
        TYPED_ARRAY_LENGTH_PROPERTY.to_owned(),
        Property::non_enumerable(Value::Number(length as f64)),
    );
    // Keep a non-configurable, non-writable own `length` for ergonomic
    // `array.length` reads (the prototype accessor also covers detach).
    object.define_property(
        "length".to_owned(),
        Property::data(Value::Number(length as f64), false, false, false),
    );

    // Persist element bytes into the buffer and materialize index properties.
    let element = bytes_per_element(native);
    let mut bytes = array_buffer::array_buffer_bytes(&buffer);
    for (index, value) in values.into_iter().enumerate() {
        write_element(native, &mut bytes, byte_offset + index * element, &value);
        object.define_property(index.to_string(), Property::data(value, true, true, false));
    }
    array_buffer::set_array_buffer_bytes(&buffer, bytes);
}

// --- byte <-> element encoding ----------------------------------------------

fn read_elements(native: NativeFunction, bytes: &[u8], offset: usize, length: usize) -> Vec<Value> {
    let element = bytes_per_element(native);
    (0..length)
        .map(|index| read_element(native, bytes, offset + index * element))
        .collect()
}

fn read_element(native: NativeFunction, bytes: &[u8], byte_index: usize) -> Value {
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
            Value::BigInt(num_bigint::BigInt::from(i64::from_le_bytes(buf)))
        }
        NativeFunction::BigUint64Array => {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(slice);
            Value::BigInt(num_bigint::BigInt::from(u64::from_le_bytes(buf)))
        }
        _ => zero_value(native),
    }
}

fn write_element(native: NativeFunction, bytes: &mut [u8], byte_index: usize, value: &Value) {
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
            let modulo = num_bigint::BigInt::from(1u128 << 64);
            let wrapped = ((big % &modulo) + &modulo) % &modulo;
            wrapped.to_u64().map(|value| value as i64).unwrap_or(0)
        }
        _ => 0,
    }
}

fn zero_value(native: NativeFunction) -> Value {
    if is_big_int_kind(native) {
        Value::BigInt(num_bigint::BigInt::from(0))
    } else {
        Value::Number(0.0)
    }
}

// --- helpers -----------------------------------------------------------------

/// ToIndex: a non-negative integer, throwing RangeError otherwise.
fn to_index(value: Value, env: &mut HashMap<String, Value>) -> Result<usize, RuntimeError> {
    let number = to_number_with_env(value, env)?;
    let integer = if number.is_nan() { 0.0 } else { number.trunc() };
    if integer < 0.0 || !integer.is_finite() {
        return Err(range_error("invalid index"));
    }
    Ok(integer as usize)
}

fn range_error(message: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("RangeError: {message}"),
    }
}

fn invalid_length_error() -> RuntimeError {
    range_error("invalid typed array length")
}
