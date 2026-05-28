use std::collections::HashMap;

use crate::{ArrayRef, Function, ObjectRef, Property, RuntimeError, Value, to_number};

pub(crate) fn constructor_prototype(callee: &Value) -> Option<ObjectRef> {
    let Value::Function(function) = callee else {
        return None;
    };
    function_prototype(function)
}

pub(crate) fn object_prototype(env: &HashMap<String, Value>) -> Option<ObjectRef> {
    let Some(Value::Function(object_function)) = env.get("Object") else {
        return None;
    };
    function_prototype(object_function)
}

pub(crate) fn array_prototype(env: &HashMap<String, Value>) -> Option<ObjectRef> {
    let Some(Value::Function(array_function)) = env.get("Array") else {
        return None;
    };
    function_prototype(array_function)
}

pub(crate) fn string_prototype(env: &HashMap<String, Value>) -> Option<ObjectRef> {
    let Some(Value::Function(string_function)) = env.get("String") else {
        return None;
    };
    function_prototype(string_function)
}

pub(crate) fn function_intrinsic_prototype(env: &HashMap<String, Value>) -> Option<ObjectRef> {
    let Some(Value::Function(function_constructor)) = env.get("Function") else {
        return None;
    };
    function_prototype(function_constructor)
}

pub(crate) fn function_prototype(function: &Function) -> Option<ObjectRef> {
    match function.properties.borrow().get("prototype") {
        Some(Property {
            value: Value::Object(prototype),
            ..
        }) => Some(prototype.clone()),
        _ => None,
    }
}

pub(crate) fn value_prototype(value: Value, env: &HashMap<String, Value>) -> Option<ObjectRef> {
    match value {
        Value::Object(object) => object.prototype(),
        Value::Array(_) => array_prototype(env),
        Value::Function(_) => function_intrinsic_prototype(env),
        Value::String(_) | Value::Number(_) | Value::Boolean(_) => None,
        Value::Null | Value::Undefined => None,
    }
}

fn object_prototype_property(env: &HashMap<String, Value>, key: &str) -> Option<Value> {
    object_prototype(env).and_then(|prototype| prototype.get(key))
}

pub(crate) fn inherited_object_prototype_property(
    env: &HashMap<String, Value>,
    key: &str,
) -> Option<Value> {
    if matches!(
        key,
        "hasOwnProperty" | "isPrototypeOf" | "propertyIsEnumerable"
    ) {
        object_prototype_property(env, key)
    } else {
        None
    }
}

pub(crate) fn inherited_function_prototype_property(
    env: &HashMap<String, Value>,
    key: &str,
) -> Option<Value> {
    function_intrinsic_prototype(env)
        .and_then(|prototype| prototype.get(key))
        .or_else(|| inherited_object_prototype_property(env, key))
}

pub(crate) fn inherited_array_prototype_property(
    env: &HashMap<String, Value>,
    key: &str,
) -> Option<Value> {
    array_prototype(env)
        .and_then(|prototype| prototype.get(key))
        .or_else(|| inherited_object_prototype_property(env, key))
}

pub(crate) fn inherited_string_prototype_property(
    env: &HashMap<String, Value>,
    key: &str,
) -> Option<Value> {
    string_prototype(env)
        .and_then(|prototype| prototype.get(key))
        .or_else(|| inherited_object_prototype_property(env, key))
}

pub(crate) fn to_property_key(value: Value) -> Result<String, RuntimeError> {
    match value {
        Value::String(value) => Ok(value),
        Value::Number(number) if number.fract() == 0.0 => Ok(format!("{number:.0}")),
        Value::Number(number) => Ok(number.to_string()),
        Value::Boolean(true) => Ok("true".to_owned()),
        Value::Boolean(false) => Ok("false".to_owned()),
        Value::Null => Ok("null".to_owned()),
        Value::Undefined => Ok("undefined".to_owned()),
        Value::Function(_) | Value::Array(_) | Value::Object(_) => Err(RuntimeError {
            message: "unsupported property key".to_owned(),
        }),
    }
}

pub(crate) fn array_has_own_property(elements: &ArrayRef, key: &str) -> bool {
    key == "length"
        || key
            .parse::<usize>()
            .is_ok_and(|index| index < elements.len())
}

pub(crate) fn array_own_property_descriptor(elements: &ArrayRef, key: &str) -> Option<Property> {
    if key == "length" {
        return Some(Property {
            value: Value::Number(elements.len() as f64),
            enumerable: false,
            writable: true,
            configurable: false,
        });
    }
    let index = key.parse::<usize>().ok()?;
    elements.get(index).map(Property::enumerable)
}

pub(crate) fn array_own_property_keys(elements: &ArrayRef) -> Vec<String> {
    (0..elements.len()).map(|index| index.to_string()).collect()
}

pub(crate) fn array_own_property_names(elements: &ArrayRef) -> Vec<String> {
    let mut names = array_own_property_keys(elements);
    names.push("length".to_owned());
    names
}

pub(crate) fn function_own_property_keys(function: &Function) -> Vec<String> {
    let mut keys: Vec<_> = function
        .properties
        .borrow()
        .iter()
        .filter(|(_, property)| property.enumerable)
        .map(|(key, _)| key.clone())
        .collect();
    keys.sort();
    keys
}

pub(crate) fn function_own_property_descriptor(function: &Function, key: &str) -> Option<Property> {
    if key == "length" {
        return Some(Property {
            value: Value::Number(function.params.len() as f64),
            enumerable: false,
            writable: false,
            configurable: true,
        });
    }
    function.properties.borrow().get(key).cloned()
}

pub(crate) fn function_own_property_names(function: &Function) -> Vec<String> {
    let mut names: Vec<_> = function.properties.borrow().keys().cloned().collect();
    names.push("length".to_owned());
    names.sort();
    names
}

pub(crate) fn to_array_index(value: Value) -> Result<usize, RuntimeError> {
    let number = to_number(value)?;
    if !number.is_finite() || number < 0.0 || number.fract() != 0.0 {
        return Err(RuntimeError {
            message: "array index must be a non-negative integer".to_owned(),
        });
    }
    Ok(number as usize)
}
