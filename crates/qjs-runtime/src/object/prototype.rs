use std::collections::HashMap;

use crate::{
    RuntimeError, Value, array_has_own_property, array_prototype, boolean, call_function, date,
    error, function_intrinsic_prototype, function_own_property_descriptor, number, string,
    to_property_key, value_prototype,
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
        Some(Value::Array(elements)) => Ok(elements
            .prototype_override()
            .unwrap_or_else(|| array_prototype(env))
            .map(Value::Object)
            .unwrap_or(Value::Null)),
        Some(Value::Function(function)) => Ok(function
            .internal_prototype_override()
            .unwrap_or_else(|| function_intrinsic_prototype(env))
            .map(Value::Object)
            .unwrap_or(Value::Null)),
        _ => Err(RuntimeError {
            thrown: None,
            message: "Object.getPrototypeOf target must be an object".to_owned(),
        }),
    }
}

pub(crate) fn native_object_set_prototype_of(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let prototype = match argument_values.get(1).cloned().unwrap_or(Value::Undefined) {
        Value::Object(prototype) => Some(prototype),
        Value::Null => None,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Object.setPrototypeOf prototype must be an object or null".to_owned(),
            });
        }
    };

    match &target {
        Value::Object(object) => object.set_prototype(prototype).map_err(|()| RuntimeError {
            thrown: None,
            message: "Object.setPrototypeOf failed".to_owned(),
        })?,
        Value::Array(elements) => elements
            .set_prototype(prototype)
            .map_err(|()| RuntimeError {
                thrown: None,
                message: "Object.setPrototypeOf failed".to_owned(),
            })?,
        Value::Function(function) => {
            function
                .set_internal_prototype(prototype)
                .map_err(|()| RuntimeError {
                    thrown: None,
                    message: "Object.setPrototypeOf failed".to_owned(),
                })?
        }
        Value::String(_) | Value::Number(_) | Value::Boolean(_) => {}
        Value::Null | Value::Undefined => {
            return Err(RuntimeError {
                thrown: None,
                message: "Object.setPrototypeOf target must not be null or undefined".to_owned(),
            });
        }
    }
    Ok(target)
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
            thrown: None,
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
            thrown: None,
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
            thrown: None,
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
            } else if string::is_string_object(&object) {
                "String"
            } else if date::is_date_object(&object) {
                "Date"
            } else if error::is_error_object(&object) {
                "Error"
            } else if let Some(tag) = object.to_string_tag() {
                return Ok(Value::String(format!("[object {tag}]")));
            } else {
                "Object"
            }
        }
    };
    Ok(Value::String(format!("[object {tag}]")))
}

pub(crate) fn native_object_prototype_to_locale_string(
    this_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match this_value {
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "toLocaleString called on null or undefined".to_owned(),
        }),
        value => {
            let to_string =
                property_value(&value, "toString", env).ok_or_else(|| RuntimeError {
                    thrown: None,
                    message: "toLocaleString target does not have a toString method".to_owned(),
                })?;
            call_function(to_string, value, Vec::new(), env, false)
        }
    }
}

pub(crate) fn native_object_prototype_value_of(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match this_value {
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "valueOf called on null or undefined".to_owned(),
        }),
        Value::Boolean(_) | Value::Number(_) | Value::String(_) => {
            Ok(super::boxed_primitive(this_value, env).expect("primitive value should box"))
        }
        _ => Ok(this_value),
    }
}

fn property_value(value: &Value, key: &str, env: &HashMap<String, Value>) -> Option<Value> {
    match value {
        Value::Object(object) => object.get(key),
        Value::Function(function) => function
            .properties
            .borrow()
            .get(key)
            .map(|property| property.value.clone())
            .or_else(|| crate::function_prototype_property(function, env, key)),
        Value::Array(elements) => {
            if key == "length" {
                Some(Value::Number(elements.len() as f64))
            } else {
                key.parse::<usize>()
                    .ok()
                    .and_then(|index| elements.get(index))
                    .or_else(|| crate::array_prototype_property(elements, env, key))
            }
        }
        Value::String(value) => {
            if key == "length" {
                Some(Value::Number(value.chars().count() as f64))
            } else {
                crate::string::string_property(value, key)
                    .or_else(|| crate::inherited_string_prototype_property(env, key))
            }
        }
        Value::Boolean(_) => boolean::inherited_boolean_prototype_property(env, key),
        Value::Number(_) => number::inherited_number_prototype_property(env, key),
        Value::Null | Value::Undefined => None,
    }
}
