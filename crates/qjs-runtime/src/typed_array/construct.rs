use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, array, array_buffer,
    property_value, symbol, to_number_with_env,
};

use super::element::write_element;
use super::{
    MAX_TYPED_ARRAY_LENGTH, TypedArraySlots, bytes_per_element, coerce_element, is_big_int_kind,
    is_typed_array_object, to_typed_array_length, typed_array_kind, typed_array_name,
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

    match argument_values.first().cloned() {
        None | Some(Value::Undefined) => {
            let object = typed_array_allocation_object(function, this_value, env)?;
            initialize_from_length(&object, native, 0, env)?;
            Ok(Value::Object(object))
        }
        Some(Value::Object(source)) if symbol::is_symbol_primitive(&source) => {
            let length = to_typed_array_length(Value::Object(source), env)?;
            let object = typed_array_allocation_object(function, this_value, env)?;
            initialize_from_length(&object, native, length, env)?;
            Ok(Value::Object(object))
        }
        Some(Value::Object(source)) if is_typed_array_object(&source) => {
            let object = typed_array_allocation_object(function, this_value, env)?;
            initialize_from_typed_array(&object, native, &source, env)?;
            Ok(Value::Object(object))
        }
        Some(Value::Object(source))
            if array_buffer::is_array_buffer_or_shared_array_buffer_object(&source) =>
        {
            let object = typed_array_allocation_object(function, this_value, env)?;
            initialize_from_buffer(&object, native, source, argument_values, env)?;
            Ok(Value::Object(object))
        }
        Some(
            source @ (Value::Object(_)
            | Value::Array(_)
            | Value::Function(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Proxy(_)),
        ) => {
            let object = typed_array_allocation_object(function, this_value, env)?;
            initialize_from_object(&object, native, source, env)?;
            Ok(Value::Object(object))
        }
        Some(other) => {
            let length = to_typed_array_length(other, env)?;
            let object = typed_array_allocation_object(function, this_value, env)?;
            initialize_from_length(&object, native, length, env)?;
            Ok(Value::Object(object))
        }
    }
}

fn typed_array_allocation_object(
    function: &Function,
    this_value: Value,
    env: &mut CallEnv,
) -> Result<ObjectRef, RuntimeError> {
    match this_value {
        Value::Object(object) => Ok(object),
        _ => Ok(ObjectRef::with_prototype_slot(
            HashMap::new(),
            crate::native_construct_prototype_slot(function, env)?,
        )),
    }
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
        Value::bigint(num_bigint::BigInt::from(0))
    } else {
        Value::Number(0.0)
    };
    let values = std::iter::repeat_n(zero, length).collect();
    install_view(object, native, buffer, 0, length, false, values);
    Ok(())
}

/// `new TA(typedArray)`: element-by-element conversion into a new buffer.
fn initialize_from_typed_array(
    object: &ObjectRef,
    native: NativeFunction,
    source: &ObjectRef,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let (_, length) = super::validate_typed_array(&Value::Object(source.clone()))?;
    let source_kind = typed_array_kind(source);
    // Cross-kind BigInt/Number conversion is forbidden by ToBigInt/ToNumber.
    if is_big_int_kind(native) != is_big_int_kind(source_kind) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: cannot mix BigInt and Number typed arrays".to_owned(),
        });
    }
    let mut values = Vec::with_capacity(length);
    for index in 0..length {
        let element = property_value(Value::Object(source.clone()), &index.to_string(), env)?;
        values.push(coerce_element(native, element, env)?);
    }
    let buffer = array_buffer_for(length * bytes_per_element(native), env);
    install_view(object, native, buffer, 0, length, false, values);
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
    let buffer_byte_length = array_buffer::buffer_bytes(&buffer).len();

    let length_tracking = explicit_length.is_none() && array_buffer::is_resizable(&buffer);
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
            if !array_buffer::is_resizable(&buffer) && buffer_byte_length % element != 0 {
                return Err(range_error("buffer length is not aligned to element size"));
            }
            if offset > buffer_byte_length {
                return Err(range_error("start offset is outside the buffer bounds"));
            }
            let byte_length = buffer_byte_length - offset;
            (byte_length / element, byte_length)
        }
    };

    let _ = byte_length;
    install_view_slots(object, native, buffer, offset, length, length_tracking);
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

    let raw_values = match iterator_method {
        Value::Undefined | Value::Null => {
            let source = array::array_like_length(source, "TypedArray", env)?;
            if source.length > MAX_TYPED_ARRAY_LENGTH {
                return Err(invalid_length_error());
            }
            array::array_like_values_from_receiver(source.receiver, source.length, env)?
        }
        Value::Function(ref function)
            if function.native_kind() == Some(NativeFunction::ArrayPrototypeValues)
                && array::array_iterator_next_is_native(env) =>
        {
            match &source {
                Value::Array(array) => match array.dense_argument_values(env) {
                    Some(values) => values,
                    None => array::iterable_values_with_env(source, "TypedArray", env)?,
                },
                _ => array::iterable_values_with_env(source, "TypedArray", env)?,
            }
        }
        Value::Function(_) => array::iterable_values_with_env(source, "TypedArray", env)?,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: TypedArray @@iterator method is not callable".to_owned(),
            });
        }
    };

    let mut values = Vec::with_capacity(raw_values.len());
    for value in raw_values {
        values.push(coerce_element(native, value, env)?);
    }
    let length = values.len();
    let buffer = array_buffer_for(length * bytes_per_element(native), env);
    install_view(object, native, buffer, 0, length, false, values);
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
    install_view(&object, native, buffer, 0, length, false, values);
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

/// Writes the internal slots and backing bytes. Integer-indexed reads,
/// descriptors, and own-key enumeration are handled by typed-array exotic
/// helpers, so elements are not duplicated into the ordinary property map.
fn install_view(
    object: &ObjectRef,
    native: NativeFunction,
    buffer: ObjectRef,
    byte_offset: usize,
    length: usize,
    length_tracking: bool,
    values: Vec<Value>,
) {
    install_view_slots(
        object,
        native,
        buffer.clone(),
        byte_offset,
        length,
        length_tracking,
    );
    // Persist element bytes into the buffer.
    let element = bytes_per_element(native);
    let mut bytes = array_buffer::array_buffer_bytes(&buffer);
    for (index, value) in values.into_iter().enumerate() {
        write_element(native, &mut bytes, byte_offset + index * element, &value);
    }
    array_buffer::set_array_buffer_bytes(&buffer, bytes);
}

/// Writes the internal slots for a typed-array view over an existing backing
/// buffer without touching its bytes.
fn install_view_slots(
    object: &ObjectRef,
    native: NativeFunction,
    buffer: ObjectRef,
    byte_offset: usize,
    length: usize,
    length_tracking: bool,
) {
    let name = typed_array_name(native);
    object.install_typed_array_slots(TypedArraySlots::new(
        native,
        buffer,
        byte_offset,
        length,
        length_tracking,
    ));
    object.set_to_string_tag(name);
}

// --- static methods (%TypedArray%.from / %TypedArray%.of) --------------------

/// `%TypedArray%.of(...items)` (ES2024 23.2.2.2): constructs `new this(len)` via
/// the `this` constructor and assigns each item through `[[Set]]`, applying the
/// per-kind conversion. `this` must be a constructor.
pub(crate) fn native_typed_array_of(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let length = argument_values.len();
    let result = typed_array_create(this_value, length, true, env)?;
    for (index, value) in argument_values.iter().enumerate() {
        set_result_element(&result, index, value.clone(), env)?;
    }
    Ok(result)
}

/// `%TypedArray%.from(source [, mapfn [, thisArg]])` (ES2024 23.2.2.1):
/// constructs `new this(len)` and assigns each (optionally mapped) source
/// element through `[[Set]]`. `this` must be a constructor; a present `mapfn`
/// must be callable.
pub(crate) fn native_typed_array_from(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    crate::ensure_constructor(&this_value)?;
    let source = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let map_fn = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let this_arg = argument_values.get(2).cloned().unwrap_or(Value::Undefined);

    let mapping = match &map_fn {
        Value::Undefined => false,
        Value::Function(_) => true,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: TypedArray.from mapfn is not callable".to_owned(),
            });
        }
    };

    // Iterable sources are collected before construction; array-like sources
    // construct after LengthOfArrayLike and before indexed element reads.
    let iterator_method = match crate::symbol::iterator_symbol(env) {
        Some(symbol) => {
            crate::property_value_key(source.clone(), &crate::PropertyKey::Symbol(symbol), env)?
        }
        None => Value::Undefined,
    };
    if matches!(iterator_method, Value::Function(_)) {
        let raw_values = crate::array::iterable_values_from_method_with_env(
            source,
            iterator_method,
            "TypedArray.from",
            env,
        )?;
        let length = raw_values.len();
        let result = typed_array_create(this_value, length, true, env)?;
        for (index, value) in raw_values.into_iter().enumerate() {
            let value =
                mapped_typed_array_from_value(value, index, mapping, &map_fn, &this_arg, env)?;
            set_result_element(&result, index, value, env)?;
        }
        Ok(result)
    } else {
        let source = crate::array::array_like_length(source, "TypedArray.from", env)?;
        let result = typed_array_create(this_value, source.length, true, env)?;
        for index in 0..source.length {
            let value = property_value(source.receiver.clone(), &index.to_string(), env)?;
            let value =
                mapped_typed_array_from_value(value, index, mapping, &map_fn, &this_arg, env)?;
            set_result_element(&result, index, value, env)?;
        }
        Ok(result)
    }
}

fn mapped_typed_array_from_value(
    value: Value,
    index: usize,
    mapping: bool,
    map_fn: &Value,
    this_arg: &Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if mapping {
        crate::call_function(
            map_fn.clone(),
            this_arg.clone(),
            vec![value, Value::Number(index as f64)],
            env,
            false,
        )
    } else {
        Ok(value)
    }
}

fn typed_array_create(
    constructor: Value,
    length: usize,
    reject_immutable_buffer: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    crate::ensure_constructor(&constructor)?;
    let result = crate::construct_function(
        constructor.clone(),
        constructor,
        vec![Value::Number(length as f64)],
        env,
    )?;
    let (object, actual_length) = super::validate_typed_array(&result)?;
    if reject_immutable_buffer
        && super::typed_array_buffer(&object)
            .is_some_and(|buffer| array_buffer::is_immutable(&buffer))
    {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: ArrayBuffer is immutable".to_owned(),
        });
    }
    if actual_length < length {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: typed array constructor returned too few elements".to_owned(),
        });
    }
    Ok(result)
}

/// Assigns element `index = value` on the freshly constructed `from`/`of`
/// result through the ordinary `[[Set]]` path so a typed-array result applies
/// IntegerIndexedElementSet (and a custom-constructor result observes the set).
fn set_result_element(
    result: &Value,
    index: usize,
    value: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    crate::bytecode::set_object_property(result.clone(), index.to_string(), value, env)?;
    Ok(())
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
