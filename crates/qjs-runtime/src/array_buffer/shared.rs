use std::collections::HashMap;

use crate::{
    CallEnv, Function, NativeFunction, ObjectRef, Property, RuntimeError, Value,
    construct_function, symbol,
};

use super::{
    ensure_array_buffer_allocation_length, resizable_max_byte_length, slice_index,
    species_constructor, to_index, to_index_unbounded,
};

const SHARED_ARRAY_BUFFER_DATA_PROPERTY: &str = "\0SharedArrayBufferData";
const SHARED_ARRAY_BUFFER_MAX_BYTE_LENGTH_PROPERTY: &str = "\0SharedArrayBufferMaxByteLength";

pub(crate) fn install_shared_array_buffer(
    env: &mut CallEnv,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    prototype.set_to_string_tag("SharedArrayBuffer");
    symbol::define_well_known_to_string_tag(env, &prototype, "SharedArrayBuffer");
    let function = Function::new_native(
        Some("SharedArrayBuffer"),
        1,
        NativeFunction::SharedArrayBuffer,
        true,
    );
    prototype.define_non_enumerable("constructor".to_owned(), Value::Function(function.clone()));
    for (name, native) in [
        (
            "byteLength",
            NativeFunction::SharedArrayBufferPrototypeByteLength,
        ),
        (
            "maxByteLength",
            NativeFunction::SharedArrayBufferPrototypeMaxByteLength,
        ),
        (
            "growable",
            NativeFunction::SharedArrayBufferPrototypeGrowable,
        ),
    ] {
        prototype.define_property(
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
    prototype.define_non_enumerable(
        "grow".to_owned(),
        Value::Function(Function::new_native(
            Some("grow"),
            1,
            NativeFunction::SharedArrayBufferPrototypeGrow,
            false,
        )),
    );
    prototype.define_non_enumerable(
        "slice".to_owned(),
        Value::Function(Function::new_native(
            Some("slice"),
            2,
            NativeFunction::SharedArrayBufferPrototypeSlice,
            false,
        )),
    );
    function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(prototype)),
    );
    symbol::define_species_accessor(env, &function);
    let value = Value::Function(function);
    env.insert_realm("SharedArrayBuffer".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("SharedArrayBuffer".to_owned(), value);
    }
}

pub(crate) fn native_shared_array_buffer(
    function: &Function,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor SharedArrayBuffer requires 'new'".to_owned(),
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
            message: "RangeError: maxByteLength is smaller than SharedArrayBuffer length"
                .to_owned(),
        });
    }
    let object = ObjectRef::with_prototype_slot(
        HashMap::new(),
        crate::native_construct_prototype_slot(function, env)?,
    );
    ensure_array_buffer_allocation_length(length)?;
    if let Some(max) = max_byte_length {
        ensure_array_buffer_allocation_length(max)?;
    }
    define_data(&object, vec![0; length]);
    if let Some(max) = max_byte_length {
        define_max_byte_length(&object, max);
    }
    Ok(Value::Object(object))
}

pub(crate) fn native_shared_array_buffer_prototype_byte_length(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = object(&this_value)?;
    Ok(Value::Number(buffer_bytes(&object).len() as f64))
}

pub(crate) fn native_shared_array_buffer_prototype_max_byte_length(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = object(&this_value)?;
    Ok(Value::Number(
        max_byte_length(&object).unwrap_or_else(|| buffer_bytes(&object).len()) as f64,
    ))
}

pub(crate) fn native_shared_array_buffer_prototype_growable(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = object(&this_value)?;
    Ok(Value::Boolean(max_byte_length(&object).is_some()))
}

pub(crate) fn native_shared_array_buffer_prototype_grow(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = object(&this_value)?;
    let Some(max_byte_length) = max_byte_length(&object) else {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: SharedArrayBuffer is not growable".to_owned(),
        });
    };
    let new_length = to_index(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let current_length = buffer_bytes(&object).len();
    if new_length < current_length {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: SharedArrayBuffer cannot shrink".to_owned(),
        });
    }
    if new_length > max_byte_length {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: SharedArrayBuffer grow length exceeds maxByteLength".to_owned(),
        });
    }
    let mut bytes = buffer_bytes(&object);
    bytes.resize(new_length, 0);
    set_bytes(&object, bytes);
    Ok(Value::Undefined)
}

pub(crate) fn native_shared_array_buffer_prototype_slice(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let source_object = object(&this_value)?;
    let length = buffer_bytes(&source_object).len();
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
        env.get("SharedArrayBuffer").unwrap_or(Value::Undefined),
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
            message: "TypeError: SharedArrayBuffer species returned the receiver".to_owned(),
        });
    }
    let result = object(&result_value)?;
    if buffer_bytes(&result).len() < new_length {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: SharedArrayBuffer species result is too small".to_owned(),
        });
    }
    let bytes = buffer_bytes(&source_object);
    let slice = bytes
        .get(start..start + new_length)
        .map(<[u8]>::to_vec)
        .unwrap_or_default();
    let mut result_bytes = buffer_bytes(&result);
    result_bytes[..new_length].copy_from_slice(&slice);
    set_bytes(&result, result_bytes);
    Ok(Value::Object(result))
}

fn define_data(object: &ObjectRef, bytes: Vec<u8>) {
    // Under the agents harness a SharedArrayBuffer's bytes live in the
    // cross-thread backing so worker agents share one memory region; otherwise
    // they live in the per-object `internal_bytes` slot.
    #[cfg(feature = "agents")]
    object.set_shared_backing(crate::array_buffer::SharedBacking::new(bytes));
    #[cfg(not(feature = "agents"))]
    object.set_internal_bytes(bytes);
    object.define_property(
        SHARED_ARRAY_BUFFER_DATA_PROPERTY.to_owned(),
        Property::non_enumerable(Value::String(::std::rc::Rc::new(String::new()))),
    );
    object.set_to_string_tag("SharedArrayBuffer");
}

fn define_max_byte_length(object: &ObjectRef, max_byte_length: usize) {
    object.define_property(
        SHARED_ARRAY_BUFFER_MAX_BYTE_LENGTH_PROPERTY.to_owned(),
        Property::non_enumerable(Value::Number(max_byte_length as f64)),
    );
}

pub(crate) fn set_bytes(object: &ObjectRef, bytes: Vec<u8>) {
    #[cfg(feature = "agents")]
    if let Some(backing) = object.shared_backing() {
        backing.set(bytes);
    } else {
        object.set_internal_bytes(bytes);
    }
    #[cfg(not(feature = "agents"))]
    object.set_internal_bytes(bytes);
    object.define_property(
        SHARED_ARRAY_BUFFER_DATA_PROPERTY.to_owned(),
        Property::non_enumerable(Value::String(::std::rc::Rc::new(String::new()))),
    );
}

pub(crate) fn buffer_bytes(object: &ObjectRef) -> Vec<u8> {
    #[cfg(feature = "agents")]
    if let Some(backing) = object.shared_backing() {
        return backing.snapshot();
    }
    object.internal_bytes().unwrap_or_default()
}

/// The cross-thread backing and resizable maximum of a `SharedArrayBuffer`, for
/// `$262.agent.broadcast`. Returns `None` for a non-shared or backing-less
/// object.
#[cfg(feature = "agents")]
pub(crate) fn backing_parts(
    object: &ObjectRef,
) -> Option<(crate::array_buffer::SharedBackingRef, Option<usize>)> {
    let backing = object.shared_backing()?;
    Some((backing, max_byte_length(object)))
}

/// Builds a `SharedArrayBuffer` in `env`'s realm that wraps an existing shared
/// `backing` (received over a broadcast), so a worker agent observes the same
/// memory the main agent shared. `max` makes the buffer growable.
#[cfg(feature = "agents")]
pub(crate) fn from_backing(
    env: &CallEnv,
    backing: crate::array_buffer::SharedBackingRef,
    max: Option<usize>,
) -> ObjectRef {
    let constructor = env.get("SharedArrayBuffer").unwrap_or(Value::Undefined);
    let prototype = crate::constructor_prototype(&constructor, env);
    let object = ObjectRef::with_prototype(HashMap::new(), prototype);
    object.set_shared_backing(backing);
    object.define_property(
        SHARED_ARRAY_BUFFER_DATA_PROPERTY.to_owned(),
        Property::non_enumerable(Value::String(::std::rc::Rc::new(String::new()))),
    );
    object.set_to_string_tag("SharedArrayBuffer");
    if let Some(max) = max {
        define_max_byte_length(&object, max);
    }
    object
}

fn max_byte_length(object: &ObjectRef) -> Option<usize> {
    match object.own_property(SHARED_ARRAY_BUFFER_MAX_BYTE_LENGTH_PROPERTY) {
        Some(Property {
            value: Value::Number(length),
            ..
        }) => Some(length as usize),
        _ => None,
    }
}

pub(crate) fn is_object(object: &ObjectRef) -> bool {
    object.has_own_property(SHARED_ARRAY_BUFFER_DATA_PROPERTY)
}

fn object(value: &Value) -> Result<ObjectRef, RuntimeError> {
    match value {
        Value::Object(object) if is_object(object) => Ok(object.clone()),
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: SharedArrayBuffer method called on incompatible receiver"
                .to_owned(),
        }),
    }
}
