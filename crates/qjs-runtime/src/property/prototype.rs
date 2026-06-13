use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    ArrayRef, Function, ObjectRef, Property, Value, array_own_property_descriptor,
    array_own_property_names, symbol,
};

pub(crate) fn constructor_prototype(callee: &Value, env: &CallEnv) -> Option<ObjectRef> {
    constructor_prototype_slot(callee, env).and_then(|prototype| prototype.as_object())
}

/// The [[Prototype]] a `new`-created instance receives from a constructor's
/// `prototype` property, preserving a function-valued prototype.
pub(crate) fn constructor_prototype_slot(
    callee: &Value,
    env: &CallEnv,
) -> Option<crate::Prototype> {
    let Value::Function(function) = callee else {
        return None;
    };
    if let Some(bound) = &function.bound {
        return constructor_prototype_slot(&bound.target, env);
    }
    match function.properties.borrow().get("prototype") {
        Some(Property {
            value: Value::Object(prototype),
            ..
        }) if !symbol::is_symbol_primitive(prototype) => {
            Some(crate::Prototype::Object(prototype.clone()))
        }
        Some(Property {
            value: Value::Function(prototype),
            ..
        }) => Some(crate::Prototype::Function(prototype.clone())),
        Some(Property {
            value: Value::Array(array),
            ..
        }) => Some(crate::Prototype::Object(array_as_object_prototype(
            array, env,
        ))),
        _ => None,
    }
}

pub(crate) fn object_prototype(env: &CallEnv) -> Option<ObjectRef> {
    let Some(Value::Function(object_function)) = env.get("Object") else {
        return None;
    };
    function_prototype(&object_function)
}

pub(crate) fn array_prototype(env: &CallEnv) -> Option<ObjectRef> {
    let Some(Value::Function(array_function)) = env.get("Array") else {
        return None;
    };
    function_prototype(&array_function)
}

pub(crate) fn string_prototype(env: &CallEnv) -> Option<ObjectRef> {
    let Some(Value::Function(string_function)) = env.get("String") else {
        return None;
    };
    function_prototype(&string_function)
}

pub(crate) fn function_intrinsic_prototype(env: &CallEnv) -> Option<ObjectRef> {
    let Some(Value::Function(function_constructor)) = env.get("Function") else {
        return None;
    };
    function_prototype(&function_constructor)
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

pub(crate) fn array_as_object_prototype(array: &ArrayRef, env: &CallEnv) -> ObjectRef {
    let prototype = ObjectRef::with_prototype(
        HashMap::new(),
        array
            .prototype_override()
            .unwrap_or_else(|| array_prototype(env)),
    );
    for key in array_own_property_names(array) {
        if let Some(descriptor) = array_own_property_descriptor(array, &key) {
            prototype.define_property(key, descriptor);
        }
    }
    prototype
}

pub(crate) fn value_prototype(value: Value, env: &CallEnv) -> Option<ObjectRef> {
    match value {
        Value::Object(object) => object.prototype(),
        Value::Map(map) => map.object().prototype(),
        Value::Set(set) => set.object().prototype(),
        Value::Array(elements) => elements
            .prototype_override()
            .unwrap_or_else(|| array_prototype(env)),
        Value::Function(function) => function
            .internal_prototype_override()
            .unwrap_or_else(|| function_intrinsic_prototype(env)),
        Value::Proxy(proxy) => value_prototype(proxy.target(), env),
        Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Boolean(_) => None,
        Value::Null | Value::Undefined => None,
    }
}

/// The immediate [[Prototype]] slot of `value`, preserving a function
/// prototype. Falls back to the relevant intrinsic when a function or array has
/// no explicit override.
pub(crate) fn value_prototype_slot(value: Value, env: &CallEnv) -> Option<crate::Prototype> {
    match value {
        Value::Object(object) => object.prototype_slot(),
        Value::Map(map) => map.object().prototype_slot(),
        Value::Set(set) => set.object().prototype_slot(),
        Value::Array(elements) => match elements.prototype_slot_override() {
            Some(slot) => slot,
            None => array_prototype(env).map(crate::Prototype::Object),
        },
        Value::Function(function) => match function.internal_prototype_slot() {
            Some(slot) => slot,
            None => function_intrinsic_prototype(env).map(crate::Prototype::Object),
        },
        Value::Proxy(proxy) => value_prototype_slot(proxy.target(), env),
        Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Boolean(_) => None,
        Value::Null | Value::Undefined => None,
    }
}

fn object_prototype_property(env: &CallEnv, key: &str) -> Option<Value> {
    object_prototype(env).and_then(|prototype| prototype.get(key))
}

fn object_prototype_descriptor(env: &CallEnv, key: &str) -> Option<Property> {
    object_prototype(env).and_then(|prototype| prototype.property(key))
}

pub(crate) fn inherited_object_prototype_property(env: &CallEnv, key: &str) -> Option<Value> {
    if matches!(
        key,
        "hasOwnProperty"
            | "isPrototypeOf"
            | "propertyIsEnumerable"
            | "__lookupGetter__"
            | "__lookupSetter__"
    ) {
        object_prototype_property(env, key)
    } else {
        None
    }
}

pub(crate) fn inherited_object_prototype_descriptor(env: &CallEnv, key: &str) -> Option<Property> {
    if matches!(
        key,
        "hasOwnProperty"
            | "isPrototypeOf"
            | "propertyIsEnumerable"
            | "__lookupGetter__"
            | "__lookupSetter__"
    ) {
        object_prototype_descriptor(env, key)
    } else {
        None
    }
}

pub(crate) fn function_prototype_property(
    function: &Function,
    env: &CallEnv,
    key: &str,
) -> Option<Value> {
    function_prototype_chain_descriptor(function, env, key).map(|property| property.value)
}

/// Walks a function's [[Prototype]] chain (object or function) for the
/// descriptor `key`, resolving the implicit default to %Function.prototype%.
pub(crate) fn function_prototype_chain_descriptor(
    function: &Function,
    env: &CallEnv,
    key: &str,
) -> Option<Property> {
    match function.internal_prototype_slot() {
        Some(Some(crate::Prototype::Object(prototype))) => prototype.property(key),
        Some(Some(crate::Prototype::Function(parent))) => parent.chain_property(key),
        Some(Some(crate::Prototype::Proxy(proxy))) => proxy
            .target_result()
            .ok()
            .and_then(|target| crate::property::own_or_inherited_descriptor(target, key)),
        Some(None) => None,
        None => function_intrinsic_prototype(env).and_then(|prototype| prototype.property(key)),
    }
}

pub(crate) fn array_prototype_property(
    elements: &ArrayRef,
    env: &CallEnv,
    key: &str,
) -> Option<Value> {
    elements
        .prototype_override()
        .unwrap_or_else(|| array_prototype(env))
        .and_then(|prototype| prototype.get(key))
}

fn constructor_named_prototype(env: &CallEnv, name: &str) -> Option<ObjectRef> {
    let Some(Value::Function(function)) = env.get(name) else {
        return None;
    };
    function_prototype(&function)
}

pub(crate) fn inherited_primitive_prototype_descriptor(
    env: &CallEnv,
    constructor_name: &str,
    key: &str,
) -> Option<Property> {
    constructor_named_prototype(env, constructor_name)
        .and_then(|prototype| prototype.property(key))
        .or_else(|| inherited_object_prototype_descriptor(env, key))
}

pub(crate) fn inherited_primitive_prototype_symbol_descriptor(
    env: &CallEnv,
    constructor_name: &str,
    symbol: &ObjectRef,
) -> Option<Property> {
    constructor_named_prototype(env, constructor_name)
        .and_then(|prototype| prototype.symbol_property(symbol))
        .or_else(|| object_prototype(env).and_then(|prototype| prototype.symbol_property(symbol)))
}

pub(crate) fn inherited_string_prototype_property(env: &CallEnv, key: &str) -> Option<Value> {
    string_prototype(env)
        .and_then(|prototype| prototype.get(key))
        .or_else(|| inherited_object_prototype_property(env, key))
}
