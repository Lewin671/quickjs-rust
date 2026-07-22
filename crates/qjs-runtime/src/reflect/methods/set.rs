use crate::CallEnv;
use crate::reflect::target::ensure_reflect_object_target;
use crate::{
    ObjectRef, Property, PropertyKey, RuntimeError, Value, call_function,
    object::{
        PropertyDescriptor, define_property_descriptor_on_value_key, own_property_descriptor_key,
    },
};

pub(crate) fn native_reflect_set(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.set")?;
    let key = crate::to_property_key_value(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let value = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    let receiver = argument_values
        .get(3)
        .cloned()
        .unwrap_or_else(|| target.clone());

    Ok(Value::Boolean(ordinary_set(
        target, &key, value, receiver, env,
    )?))
}

pub(crate) fn ordinary_set(
    target: Value,
    key: &PropertyKey,
    value: Value,
    receiver: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    if let Value::Proxy(proxy) = &target {
        return crate::proxy::proxy_set(proxy.clone(), key, value, receiver, env);
    }

    // Module namespace exotic [[Set]] returns false for every property key and
    // receiver. Do not expose its virtual writable export descriptors to
    // OrdinarySetWithOwnDescriptor: those descriptors exist for reflection,
    // but they never make assignment through the namespace succeed.
    if matches!(&target, Value::Object(object) if object.is_module_namespace_exotic()) {
        return Ok(false);
    }

    if let (Value::Object(object), PropertyKey::String(key)) = (&target, key) {
        // Typed-array [[Set]] for a CanonicalNumericIndexString: when the
        // receiver is the array itself, run IntegerIndexedElementSet; when the
        // receiver differs (any type, including a primitive), a *valid* index
        // falls through to OrdinarySet (which redirects to the receiver) but an
        // *invalid* canonical index returns true without writing or consulting
        // the prototype/receiver.
        if crate::typed_array::is_typed_array_object(object) {
            if let Some(index_valid) = crate::typed_array::canonical_index_is_valid(object, key) {
                let same_receiver = matches!(&receiver, Value::Object(r) if object.ptr_eq(r));
                if same_receiver {
                    crate::typed_array::set_indexed_element(object, key, value.clone(), env)?;
                    return Ok(true);
                }
                if !index_valid {
                    return Ok(true);
                }
            }
        }
    }

    if let Some(property) = own_property_descriptor_key(target.clone(), key, env)? {
        return ordinary_set_with_descriptor(property, key, value, receiver, env);
    }

    // OrdinarySet with no own descriptor: forward to the parent's [[Set]] via
    // the [[Prototype]] slot so a Proxy in the chain dispatches its `set` trap
    // with the original receiver, and a function prototype is walked too.
    // Native Error constructors expose the realm's Error constructor as their
    // effective parent only while no explicit function [[Prototype]] override
    // is installed. A user-supplied array, Proxy, or other live prototype must
    // retain its own recursive [[Set]] behavior.
    if let Value::Function(function) = &target
        && function.internal_prototype_slot().is_none()
        && let Some(parent) = crate::error::native_error_constructor_parent(function, env)
    {
        return ordinary_set(parent, key, value, receiver, env);
    }
    if let Some(prototype) = crate::value_prototype_slot(target, env) {
        return ordinary_set(prototype.to_value(), key, value, receiver, env);
    }

    set_receiver_data_property(receiver, key, value, env)
}

fn ordinary_set_with_descriptor(
    property: Property,
    key: &PropertyKey,
    value: Value,
    receiver: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    if property.is_accessor() {
        let Some(setter) = property.set else {
            return Ok(false);
        };
        call_function(setter, receiver, vec![value], env, false)?;
        return Ok(true);
    }
    if !property.writable {
        return Ok(false);
    }
    set_receiver_data_property(receiver, key, value, env)
}

fn set_receiver_data_property(
    receiver: Value,
    key: &PropertyKey,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let PropertyKey::String(key) = key else {
        return set_receiver_symbol_data_property(receiver, key, value, env);
    };
    match receiver {
        Value::Object(object) => {
            if object.is_module_namespace_exotic() {
                let property_key = PropertyKey::String(key.to_owned());
                let _ = own_property_descriptor_key(Value::Object(object), &property_key, env)?;
                return Ok(false);
            }
            if crate::typed_array::is_typed_array_object(&object) {
                match crate::typed_array::define_indexed_element_value(
                    &object,
                    key,
                    value.clone(),
                    env,
                )? {
                    crate::typed_array::IndexedDefine::Defined => return Ok(true),
                    crate::typed_array::IndexedDefine::Rejected => return Ok(false),
                    crate::typed_array::IndexedDefine::NotIndexed => {}
                }
            }
            let descriptor = match object.own_property(key) {
                Some(existing) if !existing.writable => return Ok(false),
                Some(existing) => Property::data(
                    value,
                    existing.enumerable,
                    existing.writable,
                    existing.configurable,
                ),
                None if !object.is_extensible() => return Ok(false),
                None => Property::enumerable(value),
            };
            object.define_property(key.to_owned(), descriptor);
            Ok(true)
        }
        Value::Array(elements) => {
            // OrdinarySetWithOwnDescriptor must inspect the Receiver's own
            // descriptor before defining through it. In particular, a writable
            // data property on the original target may not overwrite an
            // accessor or non-writable property already owned by an Array
            // receiver. Route the accepted write through Array
            // [[DefineOwnProperty]] so index/length invariants remain intact.
            let receiver = Value::Array(elements);
            let property_key = PropertyKey::String(key.to_owned());
            let descriptor =
                match own_property_descriptor_key(receiver.clone(), &property_key, env)? {
                    Some(existing) if existing.is_accessor() || !existing.writable => {
                        return Ok(false);
                    }
                    Some(existing) => PropertyDescriptor::data(
                        value,
                        existing.writable,
                        existing.enumerable,
                        existing.configurable,
                    ),
                    None => PropertyDescriptor::data(value, true, true, true),
                };
            define_property_descriptor_on_value_key(receiver, property_key, descriptor, env)
        }
        Value::Function(function) => {
            let descriptor = match crate::function_own_property_descriptor(&function, key) {
                Some(existing) if !existing.writable => return Ok(false),
                Some(existing) => Property::data(
                    value,
                    existing.enumerable,
                    existing.writable,
                    existing.configurable,
                ),
                None if !function.is_extensible() => return Ok(false),
                None => Property::enumerable(value),
            };
            function
                .properties
                .borrow_mut()
                .insert(key.to_owned(), descriptor);
            Ok(true)
        }
        Value::Map(map) => {
            let object = map.object();
            let descriptor = match object.own_property(key) {
                Some(existing) if !existing.writable => return Ok(false),
                Some(existing) => Property::data(
                    value,
                    existing.enumerable,
                    existing.writable,
                    existing.configurable,
                ),
                None if !object.is_extensible() => return Ok(false),
                None => Property::enumerable(value),
            };
            object.define_property(key.to_owned(), descriptor);
            Ok(true)
        }
        Value::Set(set) => {
            let object = set.object();
            let descriptor = match object.own_property(key) {
                Some(existing) if !existing.writable => return Ok(false),
                Some(existing) => Property::data(
                    value,
                    existing.enumerable,
                    existing.writable,
                    existing.configurable,
                ),
                None if !object.is_extensible() => return Ok(false),
                None => Property::enumerable(value),
            };
            object.define_property(key.to_owned(), descriptor);
            Ok(true)
        }
        Value::Proxy(proxy) => {
            set_proxy_receiver_data_property(proxy, PropertyKey::String(key.to_owned()), value, env)
        }
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Ok(false),
    }
}

fn set_receiver_symbol_data_property(
    receiver: Value,
    key: &PropertyKey,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let PropertyKey::Symbol(symbol) = key else {
        unreachable!("symbol set helper should only receive symbol keys");
    };
    match receiver {
        Value::Object(object) => set_object_symbol_data_property(object, symbol.clone(), value),
        Value::Map(map) => set_object_symbol_data_property(map.object(), symbol.clone(), value),
        Value::Set(set) => set_object_symbol_data_property(set.object(), symbol.clone(), value),
        Value::Function(function) => {
            let descriptor = match function.own_symbol_property(symbol) {
                Some(existing) if !existing.writable => return Ok(false),
                Some(existing) => Property::data(
                    value,
                    existing.enumerable,
                    existing.writable,
                    existing.configurable,
                ),
                None if !function.is_extensible() => return Ok(false),
                None => Property::enumerable(value),
            };
            function.define_symbol_property(symbol.clone(), descriptor);
            Ok(true)
        }
        Value::Array(elements) => {
            let descriptor = match elements.own_symbol_property(symbol) {
                Some(existing) if !existing.writable => return Ok(false),
                Some(existing) => Property::data(
                    value,
                    existing.enumerable,
                    existing.writable,
                    existing.configurable,
                ),
                None if !elements.is_extensible() => return Ok(false),
                None => Property::enumerable(value),
            };
            elements.define_symbol_property(symbol.clone(), descriptor);
            Ok(true)
        }
        Value::Proxy(proxy) => {
            set_proxy_receiver_data_property(proxy, PropertyKey::Symbol(symbol.clone()), value, env)
        }
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Ok(false),
    }
}

/// OrdinarySetWithOwnDescriptor's receiver update for an exotic Proxy.
/// `[[GetOwnProperty]]` and `[[DefineOwnProperty]]` must both dispatch through
/// the receiver, and updating an existing data property exposes a value-only
/// descriptor to the `defineProperty` trap.
fn set_proxy_receiver_data_property(
    proxy: crate::proxy::ProxyRef,
    key: PropertyKey,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let receiver = Value::Proxy(proxy.clone());
    let existing =
        crate::proxy::proxy_get_own_property_descriptor(proxy, &key, env, |target, env| {
            crate::object::own_property_descriptor_key(target, &key, env)
        })?;
    let descriptor = match existing {
        Some(existing) if existing.is_accessor() || !existing.writable => return Ok(false),
        Some(_) => PropertyDescriptor::data_value(value),
        None => PropertyDescriptor::data(value, true, true, true),
    };
    define_property_descriptor_on_value_key(receiver, key, descriptor, env)
}

fn set_object_symbol_data_property(
    object: ObjectRef,
    symbol: ObjectRef,
    value: Value,
) -> Result<bool, RuntimeError> {
    let descriptor = match object.own_symbol_property(&symbol) {
        Some(existing) if !existing.writable => return Ok(false),
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
