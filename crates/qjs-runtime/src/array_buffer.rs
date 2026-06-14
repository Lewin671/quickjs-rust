use crate::CallEnv;
use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError, Value,
    construct_function, ensure_constructor, property_value, property_value_key, symbol,
    to_number_with_env,
};

mod shared;
pub(crate) use shared::{
    buffer_bytes as shared_array_buffer_bytes, install_shared_array_buffer,
    is_object as is_shared_array_buffer_object, native_shared_array_buffer,
    native_shared_array_buffer_prototype_byte_length, native_shared_array_buffer_prototype_grow,
    native_shared_array_buffer_prototype_growable,
    native_shared_array_buffer_prototype_max_byte_length,
    native_shared_array_buffer_prototype_slice, set_bytes as set_shared_array_buffer_bytes,
};

/// Internal slot holding the backing bytes of an `ArrayBuffer`, encoded as a
/// Latin-1 string (one `char` per byte). Absent on detached buffers.
pub(crate) const ARRAY_BUFFER_DATA_PROPERTY: &str = "\0ArrayBufferData";
/// Internal marker set on a detached `ArrayBuffer`. Once set, the data slot is
/// cleared and every accessor that reaches the buffer observes a detached
/// state.
pub(crate) const ARRAY_BUFFER_DETACHED_PROPERTY: &str = "\0ArrayBufferDetached";
/// Internal slot holding the maximum byte length for resizable ArrayBuffers.
pub(crate) const ARRAY_BUFFER_MAX_BYTE_LENGTH_PROPERTY: &str = "\0ArrayBufferMaxByteLength";
/// Internal marker set on immutable ArrayBuffers.
pub(crate) const ARRAY_BUFFER_IMMUTABLE_PROPERTY: &str = "\0ArrayBufferImmutable";
const MAX_ARRAY_BUFFER_LENGTH: usize = 1_000_000;
const MAX_SAFE_INTEGER_LENGTH: usize = 9_007_199_254_740_991;

pub(crate) fn install_array_buffer(
    env: &mut CallEnv,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let array_buffer_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    array_buffer_prototype.set_to_string_tag("ArrayBuffer");
    symbol::define_well_known_to_string_tag(env, &array_buffer_prototype, "ArrayBuffer");
    let array_buffer_function =
        Function::new_native(Some("ArrayBuffer"), 1, NativeFunction::ArrayBuffer, true);
    array_buffer_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(array_buffer_function.clone()),
    );
    array_buffer_prototype.define_property(
        "byteLength".to_owned(),
        Property::accessor(
            Some(Value::Function(Function::new_native(
                Some("get byteLength"),
                0,
                NativeFunction::ArrayBufferPrototypeByteLength,
                false,
            ))),
            None,
            false,
            true,
        ),
    );
    for (name, native) in [
        (
            "maxByteLength",
            NativeFunction::ArrayBufferPrototypeMaxByteLength,
        ),
        ("resizable", NativeFunction::ArrayBufferPrototypeResizable),
        ("detached", NativeFunction::ArrayBufferPrototypeDetached),
        ("immutable", NativeFunction::ArrayBufferPrototypeImmutable),
    ] {
        array_buffer_prototype.define_property(
            name.to_owned(),
            Property::accessor(
                Some(Value::Function(Function::new_native(
                    Some(&format!("get {name}")),
                    0,
                    native,
                    false,
                ))),
                None,
                false,
                true,
            ),
        );
    }
    array_buffer_prototype.define_non_enumerable(
        "resize".to_owned(),
        Value::Function(Function::new_native(
            Some("resize"),
            1,
            NativeFunction::ArrayBufferPrototypeResize,
            false,
        )),
    );
    array_buffer_prototype.define_non_enumerable(
        "slice".to_owned(),
        Value::Function(Function::new_native(
            Some("slice"),
            2,
            NativeFunction::ArrayBufferPrototypeSlice,
            false,
        )),
    );
    array_buffer_prototype.define_non_enumerable(
        "sliceToImmutable".to_owned(),
        Value::Function(Function::new_native(
            Some("sliceToImmutable"),
            2,
            NativeFunction::ArrayBufferPrototypeSliceToImmutable,
            false,
        )),
    );
    array_buffer_prototype.define_non_enumerable(
        "transfer".to_owned(),
        Value::Function(Function::new_native(
            Some("transfer"),
            0,
            NativeFunction::ArrayBufferPrototypeTransfer,
            false,
        )),
    );
    array_buffer_prototype.define_non_enumerable(
        "transferToFixedLength".to_owned(),
        Value::Function(Function::new_native(
            Some("transferToFixedLength"),
            0,
            NativeFunction::ArrayBufferPrototypeTransferToFixedLength,
            false,
        )),
    );
    array_buffer_prototype.define_non_enumerable(
        "transferToImmutable".to_owned(),
        Value::Function(Function::new_native(
            Some("transferToImmutable"),
            0,
            NativeFunction::ArrayBufferPrototypeTransferToImmutable,
            false,
        )),
    );
    array_buffer_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(array_buffer_prototype)),
    );
    array_buffer_function.properties.borrow_mut().insert(
        "isView".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("isView"),
            1,
            NativeFunction::ArrayBufferIsView,
            false,
        ))),
    );
    symbol::define_species_accessor(env, &array_buffer_function);

    let array_buffer_value = Value::Function(array_buffer_function);
    env.insert_realm("ArrayBuffer".to_owned(), array_buffer_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("ArrayBuffer".to_owned(), array_buffer_value);
    }
}

pub(crate) fn native_array_buffer(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor ArrayBuffer requires 'new'".to_owned(),
        });
    }
    let length = to_index_unbounded(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let max_byte_length = resizable_max_byte_length(argument_values.get(1).cloned(), env)?;
    if max_byte_length.is_some_and(|max| max < length) {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: maxByteLength is smaller than ArrayBuffer length".to_owned(),
        });
    }
    let object = match this_value {
        Value::Object(object) => object,
        _ => ObjectRef::with_prototype_slot(
            HashMap::new(),
            crate::native_construct_prototype_slot(function, env)?,
        ),
    };
    ensure_array_buffer_allocation_length(length)?;
    if let Some(max) = max_byte_length {
        ensure_array_buffer_allocation_length(max)?;
    }
    define_array_buffer_data(&object, vec![0; length]);
    if let Some(max) = max_byte_length {
        define_array_buffer_max_byte_length(&object, max);
    }
    Ok(Value::Object(object))
}

pub(crate) fn native_array_buffer_is_view(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let is_view = matches!(
        argument_values.first(),
        Some(Value::Object(object))
            if crate::typed_array::is_typed_array_object(object)
                || crate::data_view::is_data_view_object(object)
    );
    Ok(Value::Boolean(is_view))
}

pub(crate) fn native_array_buffer_prototype_byte_length(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = array_buffer_object(&this_value)?;
    // A detached buffer reports a byte length of 0 rather than throwing.
    if is_detached(&object) {
        return Ok(Value::Number(0.0));
    }
    Ok(Value::Number(array_buffer_bytes(&object).len() as f64))
}

pub(crate) fn native_array_buffer_prototype_max_byte_length(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = array_buffer_object(&this_value)?;
    if is_detached(&object) {
        return Ok(Value::Number(0.0));
    }
    Ok(Value::Number(
        array_buffer_max_byte_length(&object).unwrap_or_else(|| array_buffer_bytes(&object).len())
            as f64,
    ))
}

pub(crate) fn native_array_buffer_prototype_resizable(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = array_buffer_object(&this_value)?;
    Ok(Value::Boolean(is_resizable(&object)))
}

pub(crate) fn native_array_buffer_prototype_detached(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = array_buffer_object(&this_value)?;
    Ok(Value::Boolean(is_detached(&object)))
}

pub(crate) fn native_array_buffer_prototype_immutable(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = array_buffer_object(&this_value)?;
    Ok(Value::Boolean(is_immutable(&object)))
}

pub(crate) fn native_array_buffer_prototype_resize(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = array_buffer_object(&this_value)?;
    if is_immutable(&object) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: ArrayBuffer is immutable".to_owned(),
        });
    }
    let Some(max_byte_length) = array_buffer_max_byte_length(&object) else {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: ArrayBuffer is not resizable".to_owned(),
        });
    };
    let new_length = to_index(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if is_detached(&object) {
        return Err(detached_error());
    }
    if new_length > max_byte_length {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: ArrayBuffer resize length exceeds maxByteLength".to_owned(),
        });
    }
    let mut bytes = array_buffer_bytes(&object);
    bytes.resize(new_length, 0);
    set_array_buffer_bytes(&object, bytes);
    Ok(Value::Undefined)
}

pub(crate) fn native_array_buffer_prototype_slice(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = array_buffer_object(&this_value)?;
    if is_detached(&object) {
        return Err(detached_error());
    }
    let length = array_buffer_bytes(&object).len();
    let start = slice_index(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        length,
        0,
        env,
    )?;
    let end = slice_index(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
        length,
        env,
    )?;
    let new_length = end.saturating_sub(start);
    let constructor = species_constructor(
        this_value.clone(),
        env.get("ArrayBuffer").unwrap_or(Value::Undefined),
        env,
    )?;
    let result_value = construct_function(
        constructor.clone(),
        constructor,
        vec![Value::Number(new_length as f64)],
        env,
    )?;
    if result_value.same_value(&this_value) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: ArrayBuffer species returned the receiver".to_owned(),
        });
    }
    let result = array_buffer_object(&result_value)?;
    if array_buffer_bytes(&result).len() < new_length {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: ArrayBuffer species result is too small".to_owned(),
        });
    }
    if is_immutable(&result) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: ArrayBuffer species result is immutable".to_owned(),
        });
    }
    // Re-read the source after the user-observable index coercion above, which
    // could have detached it.
    if is_detached(&object) {
        return Err(detached_error());
    }
    let bytes = array_buffer_bytes(&object);
    let slice = bytes
        .get(start..start + new_length)
        .map(<[u8]>::to_vec)
        .unwrap_or_default();
    let mut result_bytes = array_buffer_bytes(&result);
    result_bytes[..new_length].copy_from_slice(&slice);
    set_array_buffer_bytes(&result, result_bytes);
    Ok(Value::Object(result))
}

pub(crate) fn native_array_buffer_prototype_slice_to_immutable(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = array_buffer_object(&this_value)?;
    if is_detached(&object) {
        return Err(detached_error());
    }
    let length = array_buffer_bytes(&object).len();
    let start = slice_index(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        length,
        0,
        env,
    )?;
    let end = slice_index(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
        length,
        env,
    )?;
    let new_length = end.saturating_sub(start);
    if is_detached(&object) {
        return Err(detached_error());
    }
    let source = array_buffer_bytes(&object);
    if source.len() < end {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: ArrayBuffer slice bounds exceed current length".to_owned(),
        });
    }
    let slice = source
        .get(start..start + new_length)
        .map(<[u8]>::to_vec)
        .unwrap_or_default();

    let constructor = env.get("ArrayBuffer").unwrap_or(Value::Undefined);
    let prototype = crate::constructor_prototype(&constructor, env);
    let result = ObjectRef::with_prototype(HashMap::new(), prototype);
    define_array_buffer_data(&result, slice);
    define_array_buffer_immutable(&result);
    Ok(Value::Object(result))
}

pub(crate) fn native_array_buffer_prototype_transfer(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    array_buffer_copy_and_detach(
        this_value,
        argument_values,
        env,
        TransferKind::PreserveResizable,
    )
}

pub(crate) fn native_array_buffer_prototype_transfer_to_fixed_length(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    array_buffer_copy_and_detach(this_value, argument_values, env, TransferKind::FixedLength)
}

pub(crate) fn native_array_buffer_prototype_transfer_to_immutable(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    array_buffer_copy_and_detach(this_value, argument_values, env, TransferKind::Immutable)
}

enum TransferKind {
    PreserveResizable,
    FixedLength,
    Immutable,
}

fn array_buffer_copy_and_detach(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
    kind: TransferKind,
) -> Result<Value, RuntimeError> {
    let object = array_buffer_object(&this_value)?;
    let new_length = match argument_values.first() {
        Some(Value::Undefined) | None => array_buffer_bytes(&object).len(),
        Some(value) => to_index(value.clone(), env)?,
    };
    if is_detached(&object) {
        return Err(detached_error());
    }
    if is_immutable(&object) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: ArrayBuffer is immutable".to_owned(),
        });
    }
    let source = array_buffer_bytes(&object);
    let copy_length = new_length.min(source.len());
    let mut bytes = vec![0; new_length];
    bytes[..copy_length].copy_from_slice(&source[..copy_length]);

    let constructor = env.get("ArrayBuffer").unwrap_or(Value::Undefined);
    let prototype = crate::constructor_prototype(&constructor, env);
    let result = ObjectRef::with_prototype(HashMap::new(), prototype);
    define_array_buffer_data(&result, bytes);
    match kind {
        TransferKind::PreserveResizable => {
            if let Some(max_byte_length) = array_buffer_max_byte_length(&object) {
                if new_length > max_byte_length {
                    return Err(RuntimeError {
                        thrown: None,
                        message: "RangeError: ArrayBuffer transfer length exceeds maxByteLength"
                            .to_owned(),
                    });
                }
                define_array_buffer_max_byte_length(&result, max_byte_length);
            }
        }
        TransferKind::FixedLength => {}
        TransferKind::Immutable => define_array_buffer_immutable(&result),
    }
    detach(&object);
    Ok(Value::Object(result))
}

pub(super) fn resizable_max_byte_length(
    options: Option<Value>,
    env: &mut CallEnv,
) -> Result<Option<usize>, RuntimeError> {
    let Some(options) = options else {
        return Ok(None);
    };
    if matches!(options, Value::Undefined) {
        return Ok(None);
    }
    if !is_object_value(&options) {
        return Ok(None);
    }
    let max = property_value(options, "maxByteLength", env)?;
    if matches!(max, Value::Undefined) {
        return Ok(None);
    }
    Ok(Some(to_index_unbounded(max, env)?))
}

pub(super) fn species_constructor(
    value: Value,
    default: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let constructor = property_value(value, "constructor", env)?;
    if matches!(constructor, Value::Undefined) {
        return Ok(default);
    }
    if !is_object_value(&constructor) {
        return Err(array_buffer_species_constructor_error());
    }
    let species = match symbol::species_symbol(env) {
        Some(symbol) => property_value_key(constructor, &PropertyKey::Symbol(symbol), env)?,
        None => return Ok(default),
    };
    if matches!(species, Value::Undefined | Value::Null) {
        return Ok(default);
    }
    ensure_constructor(&species).map_err(|_| array_buffer_species_constructor_error())?;
    Ok(species)
}

fn array_buffer_species_constructor_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: ArrayBuffer species constructor must be an object".to_owned(),
    }
}

fn is_object_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(object) if !symbol::is_symbol_primitive(object)
    ) || matches!(
        value,
        Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) | Value::Proxy(_)
    )
}

fn define_array_buffer_data(object: &ObjectRef, bytes: Vec<u8>) {
    object.set_internal_bytes(bytes);
    object.define_property(
        ARRAY_BUFFER_DATA_PROPERTY.to_owned(),
        Property::non_enumerable(Value::String(String::new())),
    );
    object.set_to_string_tag("ArrayBuffer");
}

fn define_array_buffer_max_byte_length(object: &ObjectRef, max_byte_length: usize) {
    object.define_property(
        ARRAY_BUFFER_MAX_BYTE_LENGTH_PROPERTY.to_owned(),
        Property::non_enumerable(Value::Number(max_byte_length as f64)),
    );
}

fn define_array_buffer_immutable(object: &ObjectRef) {
    object.define_property(
        ARRAY_BUFFER_IMMUTABLE_PROPERTY.to_owned(),
        Property::non_enumerable(Value::Boolean(true)),
    );
}

/// Whether `object` carries the `ArrayBuffer` brand (data slot or detached
/// marker), used for brand checks and `ArrayBuffer.isView` consumers.
pub(crate) fn is_array_buffer_object(object: &ObjectRef) -> bool {
    object.has_own_property(ARRAY_BUFFER_DATA_PROPERTY)
        || object.has_own_property(ARRAY_BUFFER_DETACHED_PROPERTY)
}

/// The `ArrayBuffer` receiver as an object, after a brand check.
pub(crate) fn array_buffer_object(value: &Value) -> Result<ObjectRef, RuntimeError> {
    match value {
        Value::Object(object) if is_array_buffer_object(object) => Ok(object.clone()),
        _ => Err(array_buffer_receiver_error()),
    }
}

pub(crate) fn is_array_buffer_or_shared_array_buffer_object(object: &ObjectRef) -> bool {
    is_array_buffer_object(object) || is_shared_array_buffer_object(object)
}

/// Whether `object` is a detached `ArrayBuffer`.
pub(crate) fn is_detached(object: &ObjectRef) -> bool {
    object.has_own_property(ARRAY_BUFFER_DETACHED_PROPERTY)
        || !object.has_own_property(ARRAY_BUFFER_DATA_PROPERTY)
}

pub(crate) fn is_resizable(object: &ObjectRef) -> bool {
    object.has_own_property(ARRAY_BUFFER_MAX_BYTE_LENGTH_PROPERTY)
}

pub(crate) fn is_immutable(object: &ObjectRef) -> bool {
    object.has_own_property(ARRAY_BUFFER_IMMUTABLE_PROPERTY)
}

pub(crate) fn array_buffer_max_byte_length(object: &ObjectRef) -> Option<usize> {
    match object.own_property(ARRAY_BUFFER_MAX_BYTE_LENGTH_PROPERTY) {
        Some(Property {
            value: Value::Number(length),
            ..
        }) => Some(length as usize),
        _ => None,
    }
}

/// The backing bytes of a (non-detached) `ArrayBuffer`.
pub(crate) fn array_buffer_bytes(object: &ObjectRef) -> Vec<u8> {
    if let Some(bytes) = object.internal_bytes() {
        return bytes;
    }
    match object.own_property(ARRAY_BUFFER_DATA_PROPERTY) {
        Some(Property {
            value: Value::String(data),
            ..
        }) => string_to_bytes(&data),
        _ => Vec::new(),
    }
}

pub(crate) fn buffer_byte_length(object: &ObjectRef) -> usize {
    if is_shared_array_buffer_object(object) {
        shared_array_buffer_bytes(object).len()
    } else {
        array_buffer_bytes(object).len()
    }
}

pub(crate) fn buffer_bytes(object: &ObjectRef) -> Vec<u8> {
    if is_shared_array_buffer_object(object) {
        shared_array_buffer_bytes(object)
    } else {
        array_buffer_bytes(object)
    }
}

/// Replaces the backing bytes of an `ArrayBuffer` (used by typed-array writes).
pub(crate) fn set_array_buffer_bytes(object: &ObjectRef, bytes: Vec<u8>) {
    object.set_internal_bytes(bytes);
    object.define_property(
        ARRAY_BUFFER_DATA_PROPERTY.to_owned(),
        Property::non_enumerable(Value::String(String::new())),
    );
}

pub(crate) fn set_buffer_bytes(object: &ObjectRef, bytes: Vec<u8>) {
    if is_shared_array_buffer_object(object) {
        set_shared_array_buffer_bytes(object, bytes);
    } else {
        set_array_buffer_bytes(object, bytes);
    }
}

pub(crate) fn mutate_array_buffer_bytes<T>(
    object: &ObjectRef,
    f: impl FnOnce(&mut Vec<u8>) -> T,
) -> Option<T> {
    object.with_internal_bytes_mut(f)
}

/// Builds a fresh, zero-initialized `ArrayBuffer` inheriting from
/// `%ArrayBuffer.prototype%`. Used when a TypedArray allocates its own buffer.
pub(crate) fn new_array_buffer(env: &CallEnv, length: usize) -> ObjectRef {
    let constructor = env.get("ArrayBuffer").unwrap_or(Value::Undefined);
    let prototype = crate::constructor_prototype(&constructor, env);
    let object = ObjectRef::with_prototype(HashMap::new(), prototype);
    define_array_buffer_data(&object, vec![0; length]);
    object
}

/// DetachArrayBuffer: clears the backing data and marks `object` detached.
/// Idempotent; a non-ArrayBuffer argument is ignored. Used by the Test262 host
/// `$262.detachArrayBuffer` hook.
pub(crate) fn detach(object: &ObjectRef) {
    object.clear_internal_bytes();
    object.delete_own_property(ARRAY_BUFFER_DATA_PROPERTY);
    object.define_property(
        ARRAY_BUFFER_DETACHED_PROPERTY.to_owned(),
        Property::non_enumerable(Value::Boolean(true)),
    );
}

/// Host hook backing `$262.detachArrayBuffer(buffer)`: detaches `buffer` when
/// it is an `ArrayBuffer`, returning `null` (the spec's DetachArrayBuffer
/// result) in all cases.
pub(crate) fn native_detach_array_buffer(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    if let Some(Value::Object(object)) = argument_values.first() {
        if is_array_buffer_object(object) {
            detach(object);
        }
    }
    Ok(Value::Null)
}

pub(crate) fn detached_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: ArrayBuffer is detached".to_owned(),
    }
}

fn array_buffer_receiver_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: ArrayBuffer method called on incompatible receiver".to_owned(),
    }
}

pub(super) fn to_index(value: Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
    let index = to_index_unbounded(value, env)?;
    ensure_array_buffer_allocation_length(index)?;
    Ok(index)
}

fn to_index_unbounded(value: Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
    let number = to_number_with_env(value, env)?;
    let integer = if number.is_nan() { 0.0 } else { number.trunc() };
    if integer < 0.0 || !integer.is_finite() || integer > MAX_SAFE_INTEGER_LENGTH as f64 {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid ArrayBuffer length".to_owned(),
        });
    }
    Ok(integer as usize)
}

pub(super) fn ensure_array_buffer_allocation_length(length: usize) -> Result<(), RuntimeError> {
    if length > MAX_ARRAY_BUFFER_LENGTH {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid ArrayBuffer length".to_owned(),
        });
    }
    Ok(())
}

pub(super) fn slice_index(
    value: Value,
    length: usize,
    default: usize,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(default);
    }
    let number = to_number_with_env(value, env)?;
    let integer = if number.is_nan() { 0.0 } else { number.trunc() };
    if integer < 0.0 {
        Ok((length as f64 + integer).max(0.0) as usize)
    } else {
        Ok(integer.min(length as f64) as usize)
    }
}

fn string_to_bytes(value: &str) -> Vec<u8> {
    value.chars().map(|character| character as u8).collect()
}

#[cfg(test)]
mod tests;
