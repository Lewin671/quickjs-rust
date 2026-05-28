use std::collections::HashMap;

use crate::{ArrayRef, Function, ObjectRef, Property, RuntimeError, Value};

pub(crate) fn constructor_prototype(callee: &Value) -> Option<ObjectRef> {
    let Value::Function(function) = callee else {
        return None;
    };
    if let Some(bound) = &function.bound {
        return constructor_prototype(&bound.target);
    }
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
        Value::Array(elements) => elements
            .prototype_override()
            .unwrap_or_else(|| array_prototype(env)),
        Value::Function(function) => function
            .internal_prototype_override()
            .unwrap_or_else(|| function_intrinsic_prototype(env)),
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

pub(crate) fn function_prototype_property(
    function: &Function,
    env: &HashMap<String, Value>,
    key: &str,
) -> Option<Value> {
    function
        .internal_prototype_override()
        .unwrap_or_else(|| function_intrinsic_prototype(env))
        .and_then(|prototype| prototype.get(key))
}

pub(crate) fn array_prototype_property(
    elements: &ArrayRef,
    env: &HashMap<String, Value>,
    key: &str,
) -> Option<Value> {
    elements
        .prototype_override()
        .unwrap_or_else(|| array_prototype(env))
        .and_then(|prototype| prototype.get(key))
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

pub(crate) fn has_property(
    value: Value,
    env: &HashMap<String, Value>,
    key: &str,
) -> Result<bool, RuntimeError> {
    match value {
        Value::Object(object) => Ok(object.contains_property(key)),
        Value::Array(elements) => Ok(array_has_own_property(&elements, key)
            || array_prototype_property(&elements, env, key).is_some()),
        Value::Function(function) => Ok(function_own_property_descriptor(&function, key).is_some()
            || function_prototype_property(&function, env, key).is_some()),
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Err(RuntimeError {
            message: "property target must be an object".to_owned(),
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
            writable: !elements.is_frozen(),
            configurable: false,
        });
    }
    let index = key.parse::<usize>().ok()?;
    elements
        .get(index)
        .map(|value| Property::data(value, true, !elements.is_frozen(), !elements.is_sealed()))
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
            configurable: !function.is_sealed(),
        });
    }
    function.properties.borrow().get(key).cloned()
}

pub(crate) fn function_delete_own_property(function: &Function, key: &str) -> bool {
    if key == "length" {
        return false;
    }
    let mut properties = function.properties.borrow_mut();
    if properties
        .get(key)
        .is_some_and(|property| !property.configurable)
    {
        return false;
    }
    properties.remove(key);
    true
}

pub(crate) fn function_own_property_names(function: &Function) -> Vec<String> {
    let mut names: Vec<_> = function.properties.borrow().keys().cloned().collect();
    names.push("length".to_owned());
    names.sort();
    names
}
