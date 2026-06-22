use std::collections::HashMap;

use crate::{
    ArrayRef, Function, ObjectRef, Property, Value, array_own_property_descriptor,
    array_own_property_names, symbol,
};
use crate::{CallEnv, NEW_TARGET_BINDING, NativeFunction, RuntimeError};

const CROSS_REALM_MAP_PROTOTYPE: &str = "__quickjsRustRealmMapPrototype";
const CROSS_REALM_SET_PROTOTYPE: &str = "__quickjsRustRealmSetPrototype";
const CROSS_REALM_WEAK_MAP_PROTOTYPE: &str = "__quickjsRustRealmWeakMapPrototype";
const CROSS_REALM_WEAK_SET_PROTOTYPE: &str = "__quickjsRustRealmWeakSetPrototype";

pub(crate) fn constructor_prototype(callee: &Value, env: &CallEnv) -> Option<ObjectRef> {
    constructor_prototype_slot(callee, env).and_then(|prototype| prototype.as_object())
}

pub(crate) fn native_construct_prototype_slot(
    function: &Function,
    env: &mut CallEnv,
) -> Result<Option<crate::Prototype>, RuntimeError> {
    let fallback = function_prototype(function).map(crate::Prototype::Object);
    let Some(new_target) = env.get(NEW_TARGET_BINDING) else {
        return Ok(fallback);
    };
    let prototype = prototype_value_to_slot(
        crate::property_value(new_target.clone(), "prototype", env)?,
        env,
    );
    if prototype.is_some() {
        return Ok(prototype);
    }
    if let Some(marker) = native_construct_realm_prototype_marker(function.native)
        && let Some(prototype) = marked_realm_prototype_slot(&new_target, marker, env)
    {
        return Ok(Some(prototype));
    }
    Ok(fallback)
}

fn native_construct_realm_prototype_marker(native: Option<NativeFunction>) -> Option<&'static str> {
    Some(match native? {
        NativeFunction::Map => CROSS_REALM_MAP_PROTOTYPE,
        NativeFunction::Set => CROSS_REALM_SET_PROTOTYPE,
        NativeFunction::WeakMap => CROSS_REALM_WEAK_MAP_PROTOTYPE,
        NativeFunction::WeakSet => CROSS_REALM_WEAK_SET_PROTOTYPE,
        _ => return None,
    })
}

fn marked_realm_prototype_slot(
    new_target: &Value,
    marker: &str,
    env: &CallEnv,
) -> Option<crate::Prototype> {
    match new_target {
        Value::Function(function) => function
            .own_property(marker)
            .and_then(|property| prototype_value_to_slot(property.value, env)),
        Value::Proxy(proxy) => proxy
            .target_result()
            .ok()
            .and_then(|target| marked_realm_prototype_slot(&target, marker, env)),
        _ => None,
    }
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

fn prototype_value_to_slot(value: Value, env: &CallEnv) -> Option<crate::Prototype> {
    match value {
        Value::Object(prototype) if !symbol::is_symbol_primitive(&prototype) => {
            Some(crate::Prototype::Object(prototype))
        }
        Value::Function(prototype) => Some(crate::Prototype::Function(prototype)),
        Value::Array(array) => Some(crate::Prototype::Object(array_as_object_prototype(
            &array, env,
        ))),
        Value::Proxy(prototype) => Some(crate::Prototype::Proxy(prototype)),
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
    match function_intrinsic_prototype_slot(env) {
        Some(crate::Prototype::Object(prototype)) => Some(prototype),
        _ => None,
    }
}

/// The intrinsic `%Function%` constructor as a prototype slot. The
/// `%GeneratorFunction%`, `%AsyncFunction%`, and `%AsyncGeneratorFunction%`
/// constructors are subclasses of `Function`, so their `[[Prototype]]` is the
/// `Function` constructor itself (not `%Function.prototype%`).
pub(crate) fn function_constructor_as_prototype_slot(env: &CallEnv) -> Option<crate::Prototype> {
    match env.get("Function") {
        Some(Value::Function(function_constructor)) => {
            Some(crate::Prototype::Function(function_constructor))
        }
        _ => None,
    }
}

pub(crate) fn function_intrinsic_prototype_slot(env: &CallEnv) -> Option<crate::Prototype> {
    let Some(Value::Function(function_constructor)) = env.get("Function") else {
        return None;
    };
    match function_constructor.properties.borrow().get("prototype") {
        Some(Property {
            value: Value::Object(prototype),
            ..
        }) => Some(crate::Prototype::Object(prototype.clone())),
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
        Some(Property {
            value: Value::Proxy(prototype),
            ..
        }) => Some(crate::Prototype::Proxy(prototype.clone())),
        _ => None,
    }
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
            None => function_intrinsic_prototype_slot(env),
        },
        Value::Proxy(proxy) => value_prototype_slot(proxy.target(), env),
        Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Boolean(_) => None,
        Value::Null | Value::Undefined => None,
    }
}

fn object_prototype_descriptor(env: &CallEnv, key: &str) -> Option<Property> {
    object_prototype(env).and_then(|prototype| prototype.property(key))
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

/// Walks a function's [[Prototype]] chain (object or function) for the
/// descriptor `key`, resolving the implicit default to %Function.prototype%.
pub(crate) fn function_prototype_chain_descriptor(
    function: &Function,
    env: &CallEnv,
    key: &str,
) -> Option<Property> {
    match function.internal_prototype_slot() {
        Some(Some(crate::Prototype::Object(prototype))) => prototype.property(key),
        Some(Some(crate::Prototype::Function(parent))) => parent.chain_property_with_env(key, env),
        Some(Some(crate::Prototype::Proxy(proxy))) => proxy
            .target_result()
            .ok()
            .and_then(|target| crate::property::own_or_inherited_descriptor(target, key)),
        Some(None) => None,
        None => function_intrinsic_prototype_slot(env).and_then(|prototype| match prototype {
            crate::Prototype::Object(prototype) => prototype.property(key),
            crate::Prototype::Function(prototype) => prototype.chain_property_with_env(key, env),
            crate::Prototype::Proxy(proxy) => proxy
                .target_result()
                .ok()
                .and_then(|target| crate::property::own_or_inherited_descriptor(target, key)),
        }),
    }
}

pub(crate) fn constructor_named_prototype(env: &CallEnv, name: &str) -> Option<ObjectRef> {
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
