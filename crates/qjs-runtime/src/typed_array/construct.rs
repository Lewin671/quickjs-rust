use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, array, array_buffer,
    function_prototype, property_value, to_number_with_env,
};

use super::element::{read_elements, write_element};
use super::{
    TYPED_ARRAY_BUFFER_PROPERTY, TYPED_ARRAY_BYTE_OFFSET_PROPERTY, TYPED_ARRAY_KIND_PROPERTY,
    TYPED_ARRAY_LENGTH_PROPERTY, bytes_per_element, coerce_element, is_big_int_kind,
    is_typed_array_object, to_typed_array_length, typed_array_kind, typed_array_length,
    typed_array_name,
};
use crate::CallEnv;

pub(crate) fn native_typed_array(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
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
    env: &CallEnv,
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
    env: &mut CallEnv,
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
    env: &mut CallEnv,
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
    env: &mut CallEnv,
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
fn array_buffer_for(byte_length: usize, env: &CallEnv) -> ObjectRef {
    array_buffer::new_array_buffer(env, byte_length)
}

/// Creates a fresh typed array of `native`'s kind, inheriting that kind's
/// concrete `%TA.prototype%`, backed by a new buffer holding the already-coerced
/// `values`. Index reads are materialized. Used by prototype methods that return
/// a new typed array (`map`, `filter`, `slice`, `toSorted`, `toReversed`, …).
pub(crate) fn create_with_values(
    native: NativeFunction,
    values: Vec<Value>,
    env: &CallEnv,
) -> ObjectRef {
    let prototype = concrete_prototype(native, env);
    let object = ObjectRef::with_prototype(HashMap::new(), prototype);
    let length = values.len();
    let buffer = array_buffer_for(length * bytes_per_element(native), env);
    install_view(&object, native, buffer, 0, length, values);
    object
}

/// The `%TA.prototype%` object for `native`, looked up through the global
/// constructor.
fn concrete_prototype(native: NativeFunction, env: &CallEnv) -> Option<ObjectRef> {
    let constructor = env.get(typed_array_name(native))?;
    match constructor {
        Value::Function(function) => match function.own_property("prototype") {
            Some(Property {
                value: Value::Object(prototype),
                ..
            }) => Some(prototype),
            _ => None,
        },
        _ => None,
    }
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

// --- helpers -----------------------------------------------------------------

/// ToIndex: a non-negative integer, throwing RangeError otherwise.
fn to_index(value: Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
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
