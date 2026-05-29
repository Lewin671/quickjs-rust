use std::collections::HashMap;

use crate::{ArrayRef, Function, ObjectRef, Property, Value};

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
