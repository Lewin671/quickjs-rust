use std::collections::HashMap;

use crate::{
    ArrayRef, Function, NativeFunction, ObjectRef, RuntimeError, Value,
    array::array_like_values_with_env,
};

const WEAK_SET_ENTRIES: &str = "\0weak_set_entries";
const WEAK_SET_BRAND: &str = "\0weak_set";

pub(crate) fn install_weak_set(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let weak_set_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    weak_set_prototype.set_to_string_tag("WeakSet");
    let weak_set_function = Function::new_native(Some("WeakSet"), 0, NativeFunction::WeakSet, true);
    weak_set_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(weak_set_function.clone()),
    );
    define_weak_set_prototype_function(
        &weak_set_prototype,
        "add",
        1,
        NativeFunction::WeakSetPrototypeAdd,
    );
    define_weak_set_prototype_function(
        &weak_set_prototype,
        "delete",
        1,
        NativeFunction::WeakSetPrototypeDelete,
    );
    define_weak_set_prototype_function(
        &weak_set_prototype,
        "has",
        1,
        NativeFunction::WeakSetPrototypeHas,
    );
    weak_set_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        crate::Property::non_enumerable(Value::Object(weak_set_prototype)),
    );

    let value = Value::Function(weak_set_function);
    env.insert("WeakSet".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("WeakSet".to_owned(), value);
    }
}

pub(crate) fn native_weak_set(
    function: &Function,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor WeakSet requires 'new'".to_owned(),
        });
    }

    let object = ObjectRef::with_prototype(HashMap::new(), crate::function_prototype(function));
    object.set_to_string_tag("WeakSet");
    object.define_non_enumerable(WEAK_SET_BRAND.to_owned(), Value::Boolean(true));
    object.define_non_enumerable(
        WEAK_SET_ENTRIES.to_owned(),
        Value::Array(ArrayRef::new(Vec::new())),
    );
    let weak_set = Value::Object(object);

    if let Some(iterable) = argument_values.first().cloned()
        && !matches!(iterable, Value::Undefined | Value::Null)
    {
        for value in array_like_values_with_env(iterable, "WeakSet constructor", env)? {
            weak_set_add(weak_set_object(&weak_set)?, value)?;
        }
    }

    Ok(weak_set)
}

pub(crate) fn native_weak_set_prototype_add(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let object = weak_set_object(&this_value)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    weak_set_add(object, value)?;
    Ok(this_value)
}

pub(crate) fn native_weak_set_prototype_delete(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let object = weak_set_object(&this_value)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !is_weak_set_value(&value) {
        return Ok(Value::Boolean(false));
    }
    Ok(Value::Boolean(weak_set_delete(object, &value)))
}

pub(crate) fn native_weak_set_prototype_has(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let object = weak_set_object(&this_value)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !is_weak_set_value(&value) {
        return Ok(Value::Boolean(false));
    }
    Ok(Value::Boolean(weak_set_has(object, &value)))
}

fn weak_set_object(value: &Value) -> Result<ObjectRef, RuntimeError> {
    let Value::Object(object) = value else {
        return Err(incompatible_receiver());
    };
    if matches!(
        object
            .own_property(WEAK_SET_BRAND)
            .map(|property| property.value),
        Some(Value::Boolean(true))
    ) {
        Ok(object.clone())
    } else {
        Err(incompatible_receiver())
    }
}

fn weak_set_entries(object: &ObjectRef) -> Result<ArrayRef, RuntimeError> {
    match object
        .own_property(WEAK_SET_ENTRIES)
        .map(|property| property.value)
    {
        Some(Value::Array(entries)) => Ok(entries),
        _ => Err(RuntimeError {
            thrown: None,
            message: "WeakSet is missing internal state".to_owned(),
        }),
    }
}

fn weak_set_has(object: ObjectRef, value: &Value) -> bool {
    weak_set_entries(&object)
        .ok()
        .is_some_and(|entries| entries.to_vec().iter().any(|entry| entry.same_value(value)))
}

fn weak_set_add(object: ObjectRef, value: Value) -> Result<(), RuntimeError> {
    if !is_weak_set_value(&value) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: WeakSet value must be an object".to_owned(),
        });
    }
    let entries = weak_set_entries(&object)?;
    let mut values = entries.to_vec();
    if !values.iter().any(|entry| entry.same_value(&value)) {
        values.push(value);
        entries.replace_with(values);
    }
    Ok(())
}

fn weak_set_delete(object: ObjectRef, value: &Value) -> bool {
    let Ok(entries) = weak_set_entries(&object) else {
        return false;
    };
    let mut values = entries.to_vec();
    let Some(index) = values.iter().position(|entry| entry.same_value(value)) else {
        return false;
    };
    values.remove(index);
    entries.replace_with(values);
    true
}

fn is_weak_set_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    )
}

fn incompatible_receiver() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: incompatible WeakSet receiver".to_owned(),
    }
}

fn define_weak_set_prototype_function(
    prototype: &ObjectRef,
    key: &str,
    length: usize,
    native: NativeFunction,
) {
    prototype.define_non_enumerable(
        key.to_owned(),
        Value::Function(Function::new_native(Some(key), length, native, false)),
    );
}
