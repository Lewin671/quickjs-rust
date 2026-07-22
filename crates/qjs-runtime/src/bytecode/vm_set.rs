use crate::CallEnv;
use crate::{
    ObjectRef, Property, PropertyKey, RuntimeError, Value, array_prototype, call_function,
    function_own_property_descriptor, function_prototype_chain_descriptor,
    object::define_array_length_value, value::OwnDataPropertyWrite,
};

use super::vm_props::{
    ProxyInChain, ordinary_chain_property, ordinary_chain_symbol_property,
    prototype_chain_has_typed_array, prototype_chain_needs_recursive_set,
};

pub(crate) fn set_property(
    object: Value,
    key: String,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    match object {
        Value::Object(object) => {
            if crate::symbol::is_symbol_primitive(&object) {
                return set_primitive_property("Symbol", Value::Object(object), key, value, env);
            }
            // Integer-indexed writes on a typed array route through the
            // per-kind numeric conversion and the backing buffer
            // (IntegerIndexedElementSet) before the ordinary property path.
            if crate::typed_array::is_typed_array_object(&object) {
                if let crate::typed_array::IndexedWrite::Handled =
                    crate::typed_array::set_indexed_element(&object, &key, value.clone(), env)?
                {
                    return Ok(true);
                }
            }
            match object.write_existing_own_data_property(&key, &value) {
                OwnDataPropertyWrite::Written => return Ok(true),
                OwnDataPropertyWrite::ReadOnly => return Ok(false),
                OwnDataPropertyWrite::NeedsSlowPath => {}
            }
            let receiver = Value::Object(object.clone());
            ordinary_set_object(&object, receiver, key, value, env)
        }
        Value::Function(function) => {
            let receiver = Value::Function(function.clone());
            // A flat descriptor walk cannot preserve the [[Set]] semantics of
            // first-class array prototypes, Proxies, or namespace objects.
            // Keep ordinary function chains on the compact path, but recurse
            // when an observable prototype node is reachable.
            let prototype = match function.internal_prototype_slot() {
                Some(prototype) => prototype,
                None => crate::function_intrinsic_prototype_slot(env),
            };
            if crate::typed_array::canonical_numeric_index(&key).is_some()
                && prototype_chain_has_typed_array(prototype.clone())
            {
                return crate::reflect::ordinary_set(
                    receiver.clone(),
                    &PropertyKey::String(key),
                    value,
                    receiver,
                    env,
                );
            }
            if prototype_chain_needs_recursive_set(prototype) {
                return crate::reflect::ordinary_set(
                    receiver.clone(),
                    &PropertyKey::String(key),
                    value,
                    receiver,
                    env,
                );
            }
            let inherited = function_property_for_set(&function, env, &key);
            match apply_set_step(inherited, receiver, value.clone(), env)? {
                SetStep::Done(ok) => Ok(ok),
                SetStep::WriteData => {
                    function.set_property(key, value);
                    Ok(true)
                }
            }
        }
        Value::Array(elements) => {
            if key == "length" {
                // Assignment uses OrdinarySetWithOwnDescriptor semantics: an
                // own non-writable length rejects before coercing the value,
                // even when the requested numeric length would be unchanged.
                if !elements.is_length_writable() {
                    return Ok(false);
                }
                define_array_length_value(&elements, value, env)
            } else {
                let receiver = Value::Array(elements.clone());
                let property = match crate::array_own_property_descriptor(&elements, &key) {
                    Some(property) => Some(property),
                    None => {
                        let prototype_slot = elements
                            .prototype_slot_override()
                            .unwrap_or_else(|| array_prototype(env).map(crate::Prototype::Object));
                        // A typed array reachable as a prototype owns canonical
                        // numeric indices via its exotic [[Set]]; routing through
                        // recursive OrdinarySet stops an invalid index before the
                        // array's own [[DefineOwnProperty]] creates an element.
                        if crate::typed_array::canonical_numeric_index(&key).is_some()
                            && prototype_chain_has_typed_array(prototype_slot.clone())
                        {
                            return crate::reflect::ordinary_set(
                                Value::Array(elements.clone()),
                                &PropertyKey::String(key),
                                value,
                                receiver,
                                env,
                            );
                        }
                        // No own element/property: Array, Proxy, and namespace
                        // nodes must retain their own [[Set]] behavior. An
                        // explicit function prototype also remains on the
                        // recursive path because native error constructors can
                        // have a realm-specific parent.
                        if prototype_chain_needs_recursive_set(prototype_slot.clone())
                            || matches!(&prototype_slot, Some(crate::Prototype::Function(_)))
                        {
                            return crate::reflect::ordinary_set(
                                Value::Array(elements.clone()),
                                &PropertyKey::String(key),
                                value,
                                receiver,
                                env,
                            );
                        }
                        match prototype_slot {
                            Some(crate::Prototype::Object(prototype)) => {
                                match ordinary_chain_property(&prototype, &key) {
                                    Ok(property) => property,
                                    Err(ProxyInChain) => {
                                        return crate::reflect::ordinary_set(
                                            Value::Array(elements.clone()),
                                            &PropertyKey::String(key),
                                            value,
                                            receiver,
                                            env,
                                        );
                                    }
                                }
                            }
                            Some(
                                crate::Prototype::Array(_)
                                | crate::Prototype::Function(_)
                                | crate::Prototype::Proxy(_),
                            ) => {
                                unreachable!("special prototype routed through OrdinarySet")
                            }
                            None => None,
                        }
                    }
                };
                match apply_set_step(property, receiver, value.clone(), env)? {
                    SetStep::Done(ok) => Ok(ok),
                    SetStep::WriteData => {
                        // Creating a brand-new own property requires an
                        // extensible array; an existing own writable element is
                        // overwritten as usual.
                        if !crate::array_has_own_property(&elements, &key)
                            && !elements.is_extensible()
                        {
                            return Ok(false);
                        }
                        match key.parse::<usize>() {
                            Ok(index) => elements.set(index, value),
                            Err(_) => elements.set_property(key, value),
                        };
                        Ok(true)
                    }
                }
            }
        }
        Value::Map(map) => {
            let receiver = Value::Map(map.clone());
            ordinary_set_object(&map.object(), receiver, key, value, env)
        }
        Value::Set(set) => {
            let receiver = Value::Set(set.clone());
            ordinary_set_object(&set.object(), receiver, key, value, env)
        }
        Value::Proxy(proxy) => crate::reflect::ordinary_set(
            Value::Proxy(proxy.clone()),
            &PropertyKey::String(key),
            value,
            Value::Proxy(proxy),
            env,
        ),
        // PutValue with a primitive base (number/string/boolean/bigint):
        // ToObject coerces the primitive to its wrapper, then `[[Set]]` runs
        // with the original primitive as the receiver. A setter or a Proxy in
        // the wrapper's prototype chain therefore still fires.
        Value::Number(_) => set_primitive_property("Number", object, key, value, env),
        Value::Boolean(_) => set_primitive_property("Boolean", object, key, value, env),
        Value::BigInt(_) => set_primitive_property("BigInt", object, key, value, env),
        Value::String(_) => set_primitive_property("String", object, key, value, env),
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "member assignment target is not an object".to_owned(),
        }),
    }
}

/// OrdinarySet for a primitive base. Resolves the governing descriptor through
/// the primitive wrapper's prototype chain (proxy-aware), running any setter or
/// Proxy `set` trap against the original primitive as the receiver. A data
/// write resolves to `false`: ToObject yields a fresh wrapper and the receiver
/// is the primitive, so creating an own property is unobservable (a silent
/// no-op in sloppy mode; the caller raises the strict TypeError).
fn set_primitive_property(
    constructor_name: &str,
    receiver: Value,
    key: String,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let Some(prototype) = primitive_constructor_prototype(env, constructor_name) else {
        return Ok(false);
    };
    match ordinary_chain_property(&prototype, &key) {
        Err(ProxyInChain) => crate::reflect::ordinary_set(
            Value::Object(prototype),
            &PropertyKey::String(key),
            value,
            receiver,
            env,
        ),
        Ok(descriptor) => match apply_set_step(descriptor, receiver, value, env)? {
            SetStep::Done(ok) => Ok(ok),
            SetStep::WriteData => Ok(false),
        },
    }
}

fn primitive_constructor_prototype(env: &CallEnv, constructor_name: &str) -> Option<ObjectRef> {
    if constructor_name == "Symbol"
        && let Some(prototype) = marked_global_constructor_prototype(env, "Symbol")
    {
        return Some(prototype);
    }
    crate::constructor_named_prototype(env, constructor_name)
}

fn marked_global_constructor_prototype(env: &CallEnv, constructor_name: &str) -> Option<ObjectRef> {
    let global = env
        .get("__quickjsRustDynamicFunctionRealm")
        .and_then(|value| match value {
            Value::Object(global) => Some(global),
            _ => None,
        })
        .or_else(|| {
            env.get(crate::GLOBAL_THIS_BINDING)
                .and_then(|value| match value {
                    Value::Object(global) => Some(global),
                    _ => None,
                })
        })?;
    let Some(Property {
        value: Value::Function(constructor),
        ..
    }) = global.own_property(constructor_name)
    else {
        return None;
    };
    crate::function_prototype(&constructor)
}

pub(super) fn set_property_key(
    object: Value,
    key: PropertyKey,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    match key {
        PropertyKey::String(key) => set_property(object, key, value, env),
        PropertyKey::Symbol(symbol) => set_symbol_property(object, symbol, value, env),
    }
}

fn set_symbol_property(
    object: Value,
    symbol: ObjectRef,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    match object {
        Value::Object(object) => {
            set_object_symbol_property(object.clone(), Value::Object(object), symbol, value, env)
        }
        Value::Map(map) => {
            set_object_symbol_property(map.object(), Value::Map(map), symbol, value, env)
        }
        Value::Set(set) => {
            set_object_symbol_property(set.object(), Value::Set(set), symbol, value, env)
        }
        Value::Proxy(proxy) => crate::reflect::ordinary_set(
            Value::Proxy(proxy.clone()),
            &PropertyKey::Symbol(symbol),
            value,
            Value::Proxy(proxy),
            env,
        ),
        Value::Function(function) => set_function_symbol_property(
            function.clone(),
            Value::Function(function),
            symbol,
            value,
            env,
        ),
        Value::Array(elements) => {
            set_array_symbol_property(elements.clone(), Value::Array(elements), symbol, value, env)
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "member assignment target is not an object".to_owned(),
        }),
    }
}

fn set_function_symbol_property(
    function: crate::Function,
    receiver: Value,
    symbol: ObjectRef,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    crate::reflect::ordinary_set(
        Value::Function(function),
        &PropertyKey::Symbol(symbol),
        value,
        receiver,
        env,
    )
}

fn set_object_symbol_property(
    object: ObjectRef,
    receiver: Value,
    symbol: ObjectRef,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let inherited = match ordinary_chain_symbol_property(&object, &symbol) {
        Err(ProxyInChain) => {
            return crate::reflect::ordinary_set(
                Value::Object(object.clone()),
                &PropertyKey::Symbol(symbol),
                value,
                receiver,
                env,
            );
        }
        Ok(property) => property,
    };
    match apply_set_step(inherited, receiver, value.clone(), env)? {
        SetStep::Done(ok) => return Ok(ok),
        SetStep::WriteData => {}
    }
    let descriptor = match object.own_symbol_property(&symbol) {
        Some(existing) => Property::data(
            value,
            existing.enumerable,
            existing.writable,
            existing.configurable,
        ),
        None if !object.is_extensible() => return Ok(false),
        None => Property::enumerable(value),
    };
    object.define_symbol_property(symbol, descriptor);
    Ok(true)
}

fn set_array_symbol_property(
    array: crate::ArrayRef,
    receiver: Value,
    symbol: ObjectRef,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    crate::reflect::ordinary_set(
        Value::Array(array),
        &PropertyKey::Symbol(symbol),
        value,
        receiver,
        env,
    )
}

/// Result of inspecting the resolved (own or inherited) property in the first
/// part of OrdinarySet. `Done(ok)` means the operation finished: a setter ran
/// (`true`), or it was rejected by a non-writable data property or a
/// getter-only accessor (`false`). `WriteData` means the caller should create
/// or overwrite an own data property and report success.
enum SetStep {
    Done(bool),
    WriteData,
}

/// Implements the property-inspection prelude of OrdinarySet for a resolved
/// own-or-inherited `property`. Returns whether `[[Set]]` succeeded, or signals
/// that a data write should follow.
fn apply_set_step(
    property: Option<Property>,
    receiver: Value,
    value: Value,
    env: &mut CallEnv,
) -> Result<SetStep, RuntimeError> {
    let Some(property) = property else {
        return Ok(SetStep::WriteData);
    };
    if property.is_accessor() {
        // Accessor property: succeed only when a setter exists.
        return match property.set {
            Some(setter) => {
                call_function(setter, receiver, vec![value], env, false)?;
                Ok(SetStep::Done(true))
            }
            None => Ok(SetStep::Done(false)),
        };
    }
    // Data property (own or inherited). A non-writable data property in the
    // chain rejects the write entirely; OrdinarySet otherwise falls through to
    // creating/overwriting an own data property.
    if !property.writable {
        return Ok(SetStep::Done(false));
    }
    Ok(SetStep::WriteData)
}

/// OrdinarySet for objects backed by an [`ObjectRef`] (plain objects, Map, Set
/// exotic wrappers). Honors own and inherited non-writable data properties and
/// accessors, returning the `[[Set]]` success boolean.
fn ordinary_set_object(
    object: &ObjectRef,
    receiver: Value,
    key: String,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    // A canonical numeric index whose chain reaches a typed array is governed by
    // that array's exotic [[Set]] (an invalid index returns true without writing
    // or consulting the prototype); the flat descriptor walk below would instead
    // find an accessor further up the chain. Defer to the recursive OrdinarySet.
    if crate::typed_array::canonical_numeric_index(&key).is_some()
        && prototype_chain_has_typed_array(object.prototype_slot())
    {
        return crate::reflect::ordinary_set(
            Value::Object(object.clone()),
            &PropertyKey::String(key),
            value,
            receiver,
            env,
        );
    }

    // Resolve the governing own/inherited descriptor with a single proxy-aware
    // walk. A Proxy in the chain defers to OrdinarySet so its `set` trap runs
    // against the original receiver.
    let descriptor = match ordinary_chain_property(object, &key) {
        Err(ProxyInChain) => {
            return crate::reflect::ordinary_set(
                Value::Object(object.clone()),
                &PropertyKey::String(key),
                value,
                receiver,
                env,
            );
        }
        Ok(descriptor) => descriptor,
    };
    match apply_set_step(descriptor, receiver, value.clone(), env)? {
        SetStep::Done(ok) => Ok(ok),
        SetStep::WriteData => {
            // Creating an own property (the key is only inherited or absent)
            // requires an extensible receiver. An own writable data property is
            // overwritten in place regardless of extensibility.
            if object.own_property(&key).is_none() && !object.is_extensible() {
                return Ok(false);
            }
            object.set(key, value);
            Ok(true)
        }
    }
}

pub(super) fn property_set_uses_setter(object: &Value, key: &PropertyKey, env: &CallEnv) -> bool {
    property_for_set(object, key, env).is_some_and(|property| property.set.is_some())
}

fn property_for_set(object: &Value, key: &PropertyKey, env: &CallEnv) -> Option<Property> {
    let PropertyKey::String(key) = key else {
        return symbol_property_for_set(object, key, env);
    };
    match object {
        Value::Object(object) if crate::symbol::is_symbol_primitive(object) => {
            crate::inherited_primitive_prototype_descriptor(env, "Symbol", key)
        }
        Value::Object(object) => object.property(key),
        Value::Function(function) => function_property_for_set(function, env, key),
        Value::Array(elements) => elements.property(key).or_else(|| {
            elements
                .effective_prototype_slot(env)
                .and_then(|prototype| prototype.property(key))
        }),
        Value::Map(map) => map.object().property(key),
        Value::Set(set) => set.object().property(key),
        _ => None,
    }
}

fn function_property_for_set(
    function: &crate::Function,
    env: &CallEnv,
    key: &str,
) -> Option<Property> {
    function_own_property_descriptor(function, key)
        .or_else(|| function_prototype_chain_descriptor(function, env, key))
}

fn symbol_property_for_set(object: &Value, key: &PropertyKey, env: &CallEnv) -> Option<Property> {
    let PropertyKey::Symbol(symbol) = key else {
        unreachable!("symbol property helper should only receive symbol keys");
    };
    match object {
        Value::Object(object) => object.symbol_property(symbol),
        Value::Function(function) => function.symbol_property(symbol, env),
        Value::Map(map) => map.object().symbol_property(symbol),
        Value::Set(set) => set.object().symbol_property(symbol),
        Value::Array(elements) => elements.symbol_property(symbol).or_else(|| {
            elements
                .effective_prototype_slot(env)
                .and_then(|prototype| prototype.symbol_property(symbol))
        }),
        _ => None,
    }
}
