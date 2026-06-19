use crate::{
    PropertyKey, RuntimeError, Value, array_as_object_prototype, array_has_own_property,
    array_prototype, bigint, boolean, call_function, date, error, function_intrinsic_prototype,
    function_own_property_descriptor, function_prototype, number, object, property_value,
    property_value_key, regexp, string, symbol, to_property_key_value, value_prototype_slot,
};

use super::descriptor::own_property_descriptor_key;
use crate::CallEnv;
use crate::Property;
use crate::object::PropertyDescriptor;

pub(crate) fn native_object_get_prototype_of(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if let Some(Value::Proxy(proxy)) = argument_values.first() {
        return crate::proxy::proxy_get_prototype_of(proxy.clone(), env);
    }
    match argument_values.first() {
        Some(Value::Object(object)) => Ok(object
            .prototype_slot()
            .map(|prototype| prototype.to_value())
            .unwrap_or(Value::Null)),
        Some(Value::Map(map)) => Ok(map
            .object()
            .prototype_slot()
            .map(|prototype| prototype.to_value())
            .unwrap_or(Value::Null)),
        Some(Value::Set(set)) => Ok(set
            .object()
            .prototype_slot()
            .map(|prototype| prototype.to_value())
            .unwrap_or(Value::Null)),
        Some(Value::Array(elements)) => Ok(match elements.prototype_slot_override() {
            Some(slot) => slot
                .map(|prototype| prototype.to_value())
                .unwrap_or(Value::Null),
            None => array_prototype(env)
                .map(Value::Object)
                .unwrap_or(Value::Null),
        }),
        Some(Value::Function(function)) => {
            Ok(error::native_error_constructor_parent(function, env)
                .or_else(|| match function.internal_prototype_slot() {
                    Some(slot) => slot.map(|prototype| prototype.to_value()),
                    None => function_intrinsic_prototype(env).map(Value::Object),
                })
                .unwrap_or(Value::Null))
        }
        Some(Value::Boolean(_)) => Ok(constructor_prototype_value("Boolean", env)),
        Some(Value::BigInt(_)) => Ok(constructor_prototype_value("BigInt", env)),
        Some(Value::Number(_)) => Ok(constructor_prototype_value("Number", env)),
        Some(Value::String(_)) => Ok(constructor_prototype_value("String", env)),
        _ => Err(RuntimeError {
            thrown: None,
            message: "Object.getPrototypeOf target must be an object".to_owned(),
        }),
    }
}

/// Converts a JavaScript prototype value (object or null) into a `Prototype`
/// slot, erroring on anything that is not an object or null.
fn prototype_slot_from_value(
    prototype: Value,
    operation: &str,
    env: &CallEnv,
) -> Result<Option<crate::Prototype>, RuntimeError> {
    match prototype {
        Value::Object(prototype) if symbol::is_symbol_primitive(&prototype) => Err(RuntimeError {
            thrown: None,
            message: format!("{operation} prototype must be an object or null"),
        }),
        Value::Object(prototype) => Ok(Some(crate::Prototype::Object(prototype))),
        Value::Array(array) => Ok(Some(crate::Prototype::Object(array_as_object_prototype(
            &array, env,
        )))),
        Value::Function(function) => Ok(Some(crate::Prototype::Function(function))),
        Value::Proxy(proxy) => Ok(Some(crate::Prototype::Proxy(proxy))),
        Value::Null => Ok(None),
        _ => Err(RuntimeError {
            thrown: None,
            message: format!("{operation} prototype must be an object or null"),
        }),
    }
}

/// Ordinary `[[SetPrototypeOf]]` over a value (forwarding through a Proxy to its
/// target), returning whether the assignment succeeded. Does not invoke traps.
pub(crate) fn ordinary_set_prototype_of(
    target: &Value,
    prototype: Value,
    env: &CallEnv,
) -> Result<bool, RuntimeError> {
    let prototype = prototype_slot_from_value(prototype, "Object.setPrototypeOf", env)?;
    Ok(match target {
        Value::Object(object) if symbol::is_symbol_primitive(object) => true,
        Value::Object(object) => object.set_prototype_slot(prototype).is_ok(),
        Value::Map(map) => map.object().set_prototype_slot(prototype).is_ok(),
        Value::Set(set) => set.object().set_prototype_slot(prototype).is_ok(),
        Value::Proxy(proxy) => {
            return ordinary_set_prototype_of(&proxy.target(), prototype_to_value(prototype), env);
        }
        Value::Array(elements) => elements.set_prototype_slot(prototype).is_ok(),
        Value::Function(function) => function.set_internal_prototype_slot(prototype).is_ok(),
        Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Boolean(_) => true,
        Value::Null | Value::Undefined => true,
    })
}

fn prototype_to_value(prototype: Option<crate::Prototype>) -> Value {
    prototype
        .map(|prototype| prototype.to_value())
        .unwrap_or(Value::Null)
}

pub(crate) fn native_object_set_prototype_of(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let prototype_value = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    // Validate the prototype value shape eagerly (matches spec ordering).
    let _ = prototype_slot_from_value(prototype_value.clone(), "Object.setPrototypeOf", env)?;

    let failed = || RuntimeError {
        thrown: None,
        message: "Object.setPrototypeOf failed".to_owned(),
    };
    match &target {
        Value::Proxy(proxy) => {
            if !crate::proxy::proxy_set_prototype_of(proxy.clone(), prototype_value, env)? {
                return Err(failed());
            }
        }
        Value::Null | Value::Undefined => {
            return Err(RuntimeError {
                thrown: None,
                message: "Object.setPrototypeOf target must not be null or undefined".to_owned(),
            });
        }
        _ => {
            if !ordinary_set_prototype_of(&target, prototype_value, env)? {
                return Err(failed());
            }
        }
    }
    Ok(target)
}

/// `get Object.prototype.__proto__`: B.2.2.1.1.
///
/// `let O = ToObject(RequireObjectCoercible(this)); return O.[[GetPrototypeOf]]()`.
pub(crate) fn native_object_prototype_get_proto(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    match this_value {
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "Object.prototype.__proto__ called on null or undefined".to_owned(),
        }),
        // ToObject never fails for the remaining primitive/object cases; the
        // existing getPrototypeOf logic walks the same prototype slots.
        this_value => native_object_get_prototype_of(std::slice::from_ref(&this_value), env),
    }
}

/// `set Object.prototype.__proto__`: B.2.2.1.2.
///
/// `RequireObjectCoercible(this)`; if the value is neither Object nor Null, or
/// `this` is not an Object, this is a no-op returning `undefined`; otherwise
/// `this.[[SetPrototypeOf]](value)` and throw on failure.
pub(crate) fn native_object_prototype_set_proto(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if matches!(this_value, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.prototype.__proto__ called on null or undefined".to_owned(),
        });
    }
    let proto = argument_values.first().cloned().unwrap_or(Value::Undefined);
    // Only Object or Null proto values are honored; anything else is ignored.
    let proto_is_object_or_null = match &proto {
        Value::Object(object) => !symbol::is_symbol_primitive(object),
        Value::Array(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Proxy(_)
        | Value::Null => true,
        _ => false,
    };
    if !proto_is_object_or_null {
        return Ok(Value::Undefined);
    }
    // [[SetPrototypeOf]] only applies to objects; primitive `this` is a no-op.
    if !matches!(
        this_value,
        Value::Object(_)
            | Value::Array(_)
            | Value::Function(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Proxy(_)
    ) {
        return Ok(Value::Undefined);
    }
    native_object_set_prototype_of(&[this_value, proto], env)?;
    Ok(Value::Undefined)
}

fn constructor_prototype_value(name: &str, env: &CallEnv) -> Value {
    let Some(Value::Function(function)) = env.get(name) else {
        return Value::Null;
    };
    function_prototype(&function)
        .map(Value::Object)
        .unwrap_or(Value::Null)
}

/// A value's own property descriptor (`[[GetOwnProperty]]`), dispatching a
/// Proxy's `getOwnPropertyDescriptor` trap. Used by the predicates that follow
/// the spec's "let desc be ? O.[[GetOwnProperty]](P)" step.
fn value_own_property_descriptor(
    value: Value,
    key: &PropertyKey,
    env: &mut CallEnv,
) -> Result<Option<Property>, RuntimeError> {
    if let Value::Proxy(proxy) = &value {
        crate::proxy::proxy_get_own_property_descriptor(proxy.clone(), key, env, |target, _env| {
            own_property_descriptor_key(target, key)
        })
    } else {
        own_property_descriptor_key(value, key)
    }
}

pub(crate) fn native_object_prototype_has_own_property(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let key = to_property_key_value(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    match (this_value, key) {
        (Value::Object(object), crate::PropertyKey::String(key)) => {
            Ok(Value::Boolean(object.has_own_property(&key)))
        }
        (Value::Object(object), crate::PropertyKey::Symbol(symbol)) => {
            Ok(Value::Boolean(object.has_own_symbol_property(&symbol)))
        }
        (Value::Map(map), crate::PropertyKey::String(key)) => {
            Ok(Value::Boolean(map.object().has_own_property(&key)))
        }
        (Value::Map(map), crate::PropertyKey::Symbol(symbol)) => Ok(Value::Boolean(
            map.object().has_own_symbol_property(&symbol),
        )),
        (Value::Set(set), crate::PropertyKey::String(key)) => {
            Ok(Value::Boolean(set.object().has_own_property(&key)))
        }
        (Value::Set(set), crate::PropertyKey::Symbol(symbol)) => Ok(Value::Boolean(
            set.object().has_own_symbol_property(&symbol),
        )),
        (Value::Proxy(proxy), key) => Ok(Value::Boolean(
            value_own_property_descriptor(Value::Proxy(proxy), &key, env)?.is_some(),
        )),
        (Value::Function(function), crate::PropertyKey::String(key)) => Ok(Value::Boolean(
            function_own_property_descriptor(&function, &key).is_some(),
        )),
        (Value::Function(_), crate::PropertyKey::Symbol(_)) => Ok(Value::Boolean(false)),
        (Value::Array(elements), crate::PropertyKey::String(key)) => {
            Ok(Value::Boolean(array_has_own_property(&elements, &key)))
        }
        (Value::Array(array), crate::PropertyKey::Symbol(symbol)) => {
            Ok(Value::Boolean(array.has_own_symbol_property(&symbol)))
        }
        (Value::String(value), crate::PropertyKey::String(key)) => Ok(Value::Boolean(
            crate::string::string_has_own_property(&value, &key),
        )),
        (Value::String(_), crate::PropertyKey::Symbol(_)) => Ok(Value::Boolean(false)),
        (Value::Null, _) | (Value::Undefined, _) => Err(RuntimeError {
            thrown: None,
            message: "hasOwnProperty called on null or undefined".to_owned(),
        }),
        (Value::Number(_), _) | (Value::BigInt(_), _) | (Value::Boolean(_), _) => {
            Ok(Value::Boolean(false))
        }
    }
}

pub(crate) fn native_object_prototype_property_is_enumerable(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let key = to_property_key_value(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    match this_value {
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "propertyIsEnumerable called on null or undefined".to_owned(),
        }),
        value => Ok(Value::Boolean(
            value_own_property_descriptor(value, &key, env)?
                .is_some_and(|property| property.enumerable),
        )),
    }
}

pub(crate) fn native_object_prototype_lookup_getter(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    object_prototype_lookup_accessor(this_value, argument_values, env, AccessorHalf::Getter)
}

pub(crate) fn native_object_prototype_lookup_setter(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    object_prototype_lookup_accessor(this_value, argument_values, env, AccessorHalf::Setter)
}

pub(crate) fn native_object_prototype_define_getter(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    object_prototype_define_accessor(this_value, argument_values, env, AccessorHalf::Getter)
}

pub(crate) fn native_object_prototype_define_setter(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    object_prototype_define_accessor(this_value, argument_values, env, AccessorHalf::Setter)
}

#[derive(Clone, Copy)]
enum AccessorHalf {
    Getter,
    Setter,
}

fn object_prototype_define_accessor(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
    half: AccessorHalf,
) -> Result<Value, RuntimeError> {
    if matches!(this_value, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.prototype define accessor called on null or undefined".to_owned(),
        });
    }
    let function = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if !is_callable_accessor(&function) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Object.prototype define accessor requires a callable".to_owned(),
        });
    }
    let key = to_property_key_value(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let target = super::boxed_primitive(this_value.clone(), env).unwrap_or(this_value);
    let descriptor = match half {
        AccessorHalf::Getter => PropertyDescriptor::accessor_get(function, true, true),
        AccessorHalf::Setter => PropertyDescriptor::accessor_set(function, true, true),
    };
    if !object::define_property_descriptor_on_value_key(target, key, descriptor, env)? {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.prototype define accessor failed".to_owned(),
        });
    }
    Ok(Value::Undefined)
}

fn is_callable_accessor(value: &Value) -> bool {
    match value {
        Value::Function(_) => true,
        Value::Proxy(proxy) => crate::proxy::proxy_is_callable(proxy),
        _ => false,
    }
}

fn object_prototype_lookup_accessor(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
    half: AccessorHalf,
) -> Result<Value, RuntimeError> {
    if matches!(this_value, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.prototype lookup accessor called on null or undefined".to_owned(),
        });
    }
    let key = to_property_key_value(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let mut value = super::boxed_primitive(this_value.clone(), env).unwrap_or(this_value);
    loop {
        if let Some(property) = lookup_own_property_descriptor(value.clone(), &key, env)? {
            return Ok(lookup_accessor_half(property, half).unwrap_or(Value::Undefined));
        }
        match lookup_prototype_value(value, env)? {
            Value::Null => return Ok(Value::Undefined),
            prototype => value = prototype,
        }
    }
}

fn lookup_own_property_descriptor(
    value: Value,
    key: &PropertyKey,
    env: &mut CallEnv,
) -> Result<Option<Property>, RuntimeError> {
    if let Value::Proxy(proxy) = &value {
        crate::proxy::proxy_get_own_property_descriptor(proxy.clone(), key, env, |target, _env| {
            own_property_descriptor_key(target, key)
        })
    } else {
        own_property_descriptor_key(value, key)
    }
}

fn lookup_prototype_value(value: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    if let Value::Proxy(proxy) = value {
        return crate::proxy::proxy_get_prototype_of(proxy, env);
    }
    Ok(value_prototype_slot(value, env)
        .map(|prototype| prototype.to_value())
        .unwrap_or(Value::Null))
}

fn lookup_accessor_half(property: Property, half: AccessorHalf) -> Option<Value> {
    if !property.accessor {
        return None;
    }
    match half {
        AccessorHalf::Getter => property.get,
        AccessorHalf::Setter => property.set,
    }
}

pub(crate) fn native_object_prototype_is_prototype_of(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let mut target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !is_prototype_object(&target) {
        return Ok(Value::Boolean(false));
    }
    if matches!(this_value, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "isPrototypeOf called on non-object".to_owned(),
        });
    }
    loop {
        target = immediate_prototype_value(target, env)?;
        if matches!(target, Value::Null) {
            return Ok(Value::Boolean(false));
        }
        if target.same_value(&this_value) {
            return Ok(Value::Boolean(true));
        }
    }
}

fn is_prototype_object(value: &Value) -> bool {
    match value {
        Value::Object(object) => !symbol::is_symbol_primitive(object),
        Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) | Value::Proxy(_) => {
            true
        }
        _ => false,
    }
}

fn immediate_prototype_value(value: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    if let Value::Proxy(proxy) = value {
        return crate::proxy::proxy_get_prototype_of(proxy, env);
    }
    Ok(value_prototype_slot(value, env)
        .map(|prototype| prototype.to_value())
        .unwrap_or(Value::Null))
}

pub(crate) fn native_object_prototype_to_string(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let tag = builtin_to_string_tag(this_value.clone())?;
    let tag = match symbol::to_string_tag_symbol(env) {
        Some(symbol) => match property_value_key(this_value, &PropertyKey::Symbol(symbol), env)? {
            Value::String(tag) => tag.to_string(),
            _ => tag,
        },
        None => tag,
    };
    Ok(Value::String(format!("[object {tag}]").into()))
}

fn builtin_to_string_tag(value: Value) -> Result<String, RuntimeError> {
    Ok(match value {
        Value::Undefined => "Undefined".to_owned(),
        Value::Null => "Null".to_owned(),
        Value::Array(_) => "Array".to_owned(),
        Value::Function(_) => "Function".to_owned(),
        Value::Map(_) | Value::Set(_) => "Object".to_owned(),
        Value::Proxy(proxy) => {
            if crate::proxy::proxy_target_is_array_result(&proxy)? {
                "Array".to_owned()
            } else if crate::proxy::proxy_is_callable(&proxy) {
                "Function".to_owned()
            } else {
                "Object".to_owned()
            }
        }
        Value::String(_) => "String".to_owned(),
        Value::Number(_) => "Number".to_owned(),
        Value::BigInt(_) => "Object".to_owned(),
        Value::Boolean(_) => "Boolean".to_owned(),
        Value::Object(object) => {
            if boolean::is_boolean_object(&object) {
                "Boolean".to_owned()
            } else if bigint::is_bigint_object(&object) {
                "Object".to_owned()
            } else if number::is_number_object(&object) {
                "Number".to_owned()
            } else if string::is_string_object(&object) {
                "String".to_owned()
            } else if date::is_date_object(&object) {
                "Date".to_owned()
            } else if regexp::regexp_is_regexp(&Value::Object(object.clone())) {
                "RegExp".to_owned()
            } else if error::is_error_object(&object) {
                "Error".to_owned()
            } else if object.to_string_tag().as_deref() == Some("Arguments") {
                "Arguments".to_owned()
            } else if object.is_array_prototype_exotic() {
                "Array".to_owned()
            } else {
                "Object".to_owned()
            }
        }
    })
}

pub(crate) fn native_object_prototype_to_locale_string(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    match this_value {
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "toLocaleString called on null or undefined".to_owned(),
        }),
        value => {
            let to_string = property_value(value.clone(), "toString", env)?;
            call_function(to_string, value, Vec::new(), env, false)
        }
    }
}

pub(crate) fn native_object_prototype_value_of(
    this_value: Value,
    env: &CallEnv,
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
