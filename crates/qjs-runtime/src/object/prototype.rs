use std::collections::HashMap;

use crate::{
    RuntimeError, Value, array_has_own_property, array_prototype, boolean,
    function_intrinsic_prototype, function_own_property_descriptor, number, to_property_key,
    value_prototype,
};

use super::descriptor::own_property_descriptor;

pub(crate) fn native_object_get_prototype_of(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match argument_values.first() {
        Some(Value::Object(object)) => {
            Ok(object.prototype().map(Value::Object).unwrap_or(Value::Null))
        }
        Some(Value::Array(_)) => Ok(array_prototype(env)
            .map(Value::Object)
            .unwrap_or(Value::Null)),
        Some(Value::Function(_)) => Ok(function_intrinsic_prototype(env)
            .map(Value::Object)
            .unwrap_or(Value::Null)),
        _ => Err(RuntimeError {
            message: "Object.getPrototypeOf target must be an object".to_owned(),
        }),
    }
}

pub(crate) fn native_object_prototype_has_own_property(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let key = to_property_key(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    match this_value {
        Value::Object(object) => Ok(Value::Boolean(object.has_own_property(&key))),
        Value::Function(function) => Ok(Value::Boolean(
            function_own_property_descriptor(&function, &key).is_some(),
        )),
        Value::Array(elements) => Ok(Value::Boolean(array_has_own_property(&elements, &key))),
        Value::String(value) => Ok(Value::Boolean(crate::string::string_has_own_property(
            &value, &key,
        ))),
        Value::Null | Value::Undefined => Err(RuntimeError {
            message: "hasOwnProperty called on null or undefined".to_owned(),
        }),
        Value::Number(_) | Value::Boolean(_) => Ok(Value::Boolean(false)),
    }
}

pub(crate) fn native_object_prototype_property_is_enumerable(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let key = to_property_key(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    match this_value {
        Value::Null | Value::Undefined => Err(RuntimeError {
            message: "propertyIsEnumerable called on null or undefined".to_owned(),
        }),
        value => Ok(Value::Boolean(
            own_property_descriptor(value, &key)?.is_some_and(|property| property.enumerable),
        )),
    }
}

pub(crate) fn native_object_prototype_is_prototype_of(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let Some(target_prototype) = value_prototype(target, env) else {
        return Ok(Value::Boolean(false));
    };
    let Value::Object(prototype) = this_value else {
        return Err(RuntimeError {
            message: "isPrototypeOf called on non-object".to_owned(),
        });
    };
    Ok(Value::Boolean(
        target_prototype.ptr_eq(&prototype) || target_prototype.has_prototype(&prototype),
    ))
}

pub(crate) fn native_object_prototype_to_string(this_value: Value) -> Result<Value, RuntimeError> {
    let tag = match this_value {
        Value::Undefined => "Undefined",
        Value::Null => "Null",
        Value::Array(_) => "Array",
        Value::Function(_) => "Function",
        Value::String(_) => "String",
        Value::Number(_) => "Number",
        Value::Boolean(_) => "Boolean",
        Value::Object(object) => {
            if boolean::is_boolean_object(&object) {
                "Boolean"
            } else if number::is_number_object(&object) {
                "Number"
            } else {
                "Object"
            }
        }
    };
    Ok(Value::String(format!("[object {tag}]")))
}

pub(crate) fn native_object_prototype_value_of(this_value: Value) -> Result<Value, RuntimeError> {
    match this_value {
        Value::Null | Value::Undefined => Err(RuntimeError {
            message: "valueOf called on null or undefined".to_owned(),
        }),
        _ => Ok(this_value),
    }
}
