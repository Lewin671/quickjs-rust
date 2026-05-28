use std::collections::HashMap;

use crate::{Function, ObjectRef, Property, RuntimeError, Value, function_prototype};

use super::descriptor::native_object_define_properties;
use super::enumeration::enumerable_property_entries;

pub(crate) fn native_object_assign(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    match target {
        Value::Object(_) | Value::Function(_) => {}
        Value::Null | Value::Undefined => {
            return Err(RuntimeError {
                message: "Object.assign target must not be null or undefined".to_owned(),
            });
        }
        Value::Array(_) | Value::String(_) | Value::Number(_) | Value::Boolean(_) => {
            return Err(RuntimeError {
                message: "Object.assign primitive targets are not implemented".to_owned(),
            });
        }
    }

    for source in argument_values.iter().skip(1).cloned() {
        if matches!(source, Value::Null | Value::Undefined) {
            continue;
        }
        for (key, value) in enumerable_property_entries(source)? {
            set_property(target.clone(), key, value)?;
        }
    }
    Ok(target)
}

pub(crate) fn native_object(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    match argument_values.first() {
        Some(Value::Array(_) | Value::Function(_) | Value::Object(_)) => {
            Ok(argument_values[0].clone())
        }
        _ if is_construct => Ok(this_value),
        _ => Ok(Value::Object(ObjectRef::with_prototype(
            HashMap::new(),
            function_prototype(function),
        ))),
    }
}

pub(crate) fn native_object_create(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let object = match argument_values.first() {
        Some(Value::Object(prototype)) => Value::Object(ObjectRef::with_prototype(
            HashMap::new(),
            Some(prototype.clone()),
        )),
        Some(Value::Null) => Value::Object(ObjectRef::new(HashMap::new())),
        _ => {
            return Err(RuntimeError {
                message: "Object.create prototype must be an object or null".to_owned(),
            });
        }
    };

    if !matches!(argument_values.get(1), None | Some(Value::Undefined)) {
        native_object_define_properties(&[
            object.clone(),
            argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        ])?;
    }
    Ok(object)
}

pub(crate) fn native_object_is(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let left = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let right = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(left.same_value(&right)))
}

fn set_property(target: Value, key: String, value: Value) -> Result<(), RuntimeError> {
    match target {
        Value::Object(object) => {
            object.set(key, value);
            Ok(())
        }
        Value::Function(function) => {
            function
                .properties
                .borrow_mut()
                .insert(key, Property::enumerable(value));
            Ok(())
        }
        _ => Err(RuntimeError {
            message: "property target is not mutable".to_owned(),
        }),
    }
}
