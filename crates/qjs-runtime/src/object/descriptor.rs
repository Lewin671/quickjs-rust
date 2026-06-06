use std::collections::HashMap;

use crate::{
    ObjectRef, Property, PropertyKey, RuntimeError, Value, function_own_property_descriptor,
    function_own_symbol_property_descriptor, has_property, is_truthy, object_prototype,
    property_value, to_property_key_value,
};

use super::enumeration::{enumerable_property_entries, own_property_names};

pub(crate) fn native_object_define_property(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let key = to_property_key_value(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let descriptor = to_property_descriptor(
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
        env,
    )?;

    if !define_property_on_value_key(target.clone(), key, descriptor)? {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.defineProperty failed".to_owned(),
        });
    }
    Ok(target)
}

pub(crate) fn native_object_define_properties(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_define_property_target(&target)?;

    let descriptors = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if !matches!(
        descriptors,
        Value::Array(_) | Value::Object(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    ) {
        return Err(RuntimeError {
            thrown: None,
            message: "property descriptors must be an object".to_owned(),
        });
    }

    for (key, descriptor_value) in enumerable_property_entries(descriptors, env)? {
        let descriptor = to_property_descriptor(descriptor_value, env)?;
        if !define_property_on_value(target.clone(), key, descriptor)? {
            return Err(RuntimeError {
                thrown: None,
                message: "Object.defineProperties failed".to_owned(),
            });
        }
    }
    Ok(target)
}

pub(crate) fn native_object_get_own_property_descriptor(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let key = to_property_key_value(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let Some(property) = own_property_descriptor_key(target, &key)? else {
        return Ok(Value::Undefined);
    };
    Ok(Value::Object(property_descriptor_object(
        property,
        object_prototype(env),
    )))
}

pub(crate) fn native_object_get_own_property_descriptors(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.getOwnPropertyDescriptors target must not be null or undefined"
                .to_owned(),
        });
    }

    let prototype = object_prototype(env);
    let mut descriptors = HashMap::new();
    for key in own_property_names(target.clone()) {
        if let Some(property) = own_property_descriptor(target.clone(), &key)? {
            descriptors.insert(
                key,
                Value::Object(property_descriptor_object(property, prototype.clone())),
            );
        }
    }

    Ok(Value::Object(ObjectRef::with_prototype(
        descriptors,
        prototype,
    )))
}

pub(super) fn own_property_descriptor(
    value: Value,
    key: &str,
) -> Result<Option<Property>, RuntimeError> {
    own_property_descriptor_key(value, &PropertyKey::String(key.to_owned()))
}

pub(super) fn own_property_descriptor_key(
    value: Value,
    key: &PropertyKey,
) -> Result<Option<Property>, RuntimeError> {
    match value {
        Value::Object(object) => Ok(match key {
            PropertyKey::String(key) => object.own_property(key),
            PropertyKey::Symbol(symbol) => object.own_symbol_property(symbol),
        }),
        Value::Map(map) => Ok(match key {
            PropertyKey::String(key) => map.object().own_property(key),
            PropertyKey::Symbol(symbol) => map.object().own_symbol_property(symbol),
        }),
        Value::Set(set) => Ok(match key {
            PropertyKey::String(key) => set.object().own_property(key),
            PropertyKey::Symbol(symbol) => set.object().own_symbol_property(symbol),
        }),
        Value::Function(function) => Ok(match key {
            PropertyKey::String(key) => function_own_property_descriptor(&function, key),
            PropertyKey::Symbol(symbol) => {
                function_own_symbol_property_descriptor(&function, symbol)
            }
        }),
        Value::Array(elements) => Ok(match key {
            PropertyKey::String(key) => crate::array_own_property_descriptor(&elements, key),
            PropertyKey::Symbol(symbol) => elements.own_symbol_property(symbol),
        }),
        Value::String(value) => Ok(match key {
            PropertyKey::String(key) => crate::string::string_own_property_descriptor(&value, key),
            PropertyKey::Symbol(_) => None,
        }),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Ok(None),
    }
}

pub(crate) fn define_property_on_value(
    target: Value,
    key: String,
    descriptor: Property,
) -> Result<bool, RuntimeError> {
    define_property_on_value_key(target, PropertyKey::String(key), descriptor)
}

pub(crate) fn define_property_on_value_key(
    target: Value,
    key: PropertyKey,
    descriptor: Property,
) -> Result<bool, RuntimeError> {
    let key = match key {
        PropertyKey::String(key) => key,
        PropertyKey::Symbol(symbol) => {
            return define_symbol_property_on_value(target, symbol, descriptor);
        }
    };
    match &target {
        Value::Object(object) => {
            if !object.has_own_property(&key) && !object.is_extensible() {
                return Ok(false);
            }
            if object
                .own_property(&key)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            object.define_property(key, descriptor);
            Ok(true)
        }
        Value::Map(map) => {
            let object = map.object();
            if !object.has_own_property(&key) && !object.is_extensible() {
                return Ok(false);
            }
            if object
                .own_property(&key)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            object.define_property(key, descriptor);
            Ok(true)
        }
        Value::Set(set) => {
            let object = set.object();
            if !object.has_own_property(&key) && !object.is_extensible() {
                return Ok(false);
            }
            if object
                .own_property(&key)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            object.define_property(key, descriptor);
            Ok(true)
        }
        Value::Function(function) => {
            let existing = function_own_property_descriptor(function, &key);
            if existing.is_none() && !function.is_extensible() {
                return Ok(false);
            }
            if existing.is_some_and(|property| !is_compatible_descriptor(&property, &descriptor)) {
                return Ok(false);
            }
            function.properties.borrow_mut().insert(key, descriptor);
            Ok(true)
        }
        Value::Array(elements) => {
            let existing = crate::array_own_property_descriptor(elements, &key);
            if existing.is_none() && !elements.is_extensible() {
                return Ok(false);
            }
            if existing.is_some_and(|property| !is_compatible_descriptor(&property, &descriptor)) {
                return Ok(false);
            }
            if key == "length" {
                if !matches!(descriptor.value, Value::Undefined) {
                    elements.set_len(crate::to_length(descriptor.value)?);
                }
                elements.set_length_writable(descriptor.writable);
            } else {
                elements.define_property(key, descriptor);
            }
            Ok(true)
        }
        _ => {
            ensure_define_property_target(&target)?;
            unreachable!("define property target validation should reject unsupported values")
        }
    }
}

fn define_symbol_property_on_value(
    target: Value,
    symbol: ObjectRef,
    descriptor: Property,
) -> Result<bool, RuntimeError> {
    match &target {
        Value::Object(object) => {
            if !object.has_own_symbol_property(&symbol) && !object.is_extensible() {
                return Ok(false);
            }
            if object
                .own_symbol_property(&symbol)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            object.define_symbol_property(symbol, descriptor);
            Ok(true)
        }
        Value::Map(map) => {
            let object = map.object();
            if !object.has_own_symbol_property(&symbol) && !object.is_extensible() {
                return Ok(false);
            }
            if object
                .own_symbol_property(&symbol)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            object.define_symbol_property(symbol, descriptor);
            Ok(true)
        }
        Value::Set(set) => {
            let object = set.object();
            if !object.has_own_symbol_property(&symbol) && !object.is_extensible() {
                return Ok(false);
            }
            if object
                .own_symbol_property(&symbol)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            object.define_symbol_property(symbol, descriptor);
            Ok(true)
        }
        Value::Function(function) => {
            if !function.has_own_symbol_property(&symbol) && !function.is_extensible() {
                return Ok(false);
            }
            if function
                .own_symbol_property(&symbol)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            function.define_symbol_property(symbol, descriptor);
            Ok(true)
        }
        Value::Array(elements) => {
            if !elements.has_own_symbol_property(&symbol) && !elements.is_extensible() {
                return Ok(false);
            }
            if elements
                .own_symbol_property(&symbol)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            elements.define_symbol_property(symbol, descriptor);
            Ok(true)
        }
        _ => {
            ensure_define_property_target(&target)?;
            Ok(false)
        }
    }
}

fn is_compatible_descriptor(existing: &Property, descriptor: &Property) -> bool {
    if existing.configurable {
        return true;
    }
    if descriptor.configurable {
        return false;
    }
    existing.writable || !descriptor.writable
}

fn ensure_define_property_target(target: &Value) -> Result<(), RuntimeError> {
    match target {
        Value::Object(_) | Value::Function(_) | Value::Array(_) | Value::Map(_) | Value::Set(_) => {
            Ok(())
        }
        Value::String(_) | Value::Number(_) | Value::Boolean(_) => Err(RuntimeError {
            thrown: None,
            message: "Object.defineProperty primitive targets are not implemented".to_owned(),
        }),
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "Object.defineProperty target must be an object".to_owned(),
        }),
    }
}

pub(crate) fn to_property_descriptor(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Property, RuntimeError> {
    if !matches!(
        value,
        Value::Object(_) | Value::Function(_) | Value::Array(_) | Value::Map(_) | Value::Set(_)
    ) {
        return Err(RuntimeError {
            thrown: None,
            message: "property descriptor must be an object".to_owned(),
        });
    }

    let enumerable = descriptor_bool(value.clone(), "enumerable", env)?;
    let configurable = descriptor_bool(value.clone(), "configurable", env)?;
    let has_value = has_property(value.clone(), env, "value")?;
    let descriptor_value = if has_value {
        property_value(value.clone(), "value", env)?
    } else {
        Value::Undefined
    };
    let has_writable = has_property(value.clone(), env, "writable")?;
    let writable = if has_writable {
        is_truthy(&property_value(value.clone(), "writable", env)?)
    } else {
        false
    };
    let has_get = has_property(value.clone(), env, "get")?;
    let get = if has_get {
        accessor_function(property_value(value.clone(), "get", env)?, "get")?
    } else {
        None
    };
    let has_set = has_property(value.clone(), env, "set")?;
    let set = if has_set {
        accessor_function(property_value(value.clone(), "set", env)?, "set")?
    } else {
        None
    };

    if (has_get || has_set) && (has_value || has_writable) {
        return Err(RuntimeError {
            thrown: None,
            message: "property descriptor cannot mix accessor and data fields".to_owned(),
        });
    }
    if has_get || has_set {
        return Ok(Property::accessor(get, set, enumerable, configurable));
    }

    Ok(Property {
        value: descriptor_value,
        get: None,
        set: None,
        writable,
        enumerable,
        configurable,
    })
}

fn descriptor_bool(
    value: Value,
    key: &str,
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    if has_property(value.clone(), env, key)? {
        Ok(is_truthy(&property_value(value, key, env)?))
    } else {
        Ok(false)
    }
}

fn accessor_function(value: Value, name: &str) -> Result<Option<Value>, RuntimeError> {
    match value {
        Value::Undefined => Ok(None),
        Value::Function(_) => Ok(Some(value)),
        _ => Err(RuntimeError {
            thrown: None,
            message: format!("property descriptor {name} must be callable or undefined"),
        }),
    }
}

fn property_descriptor_object(property: Property, prototype: Option<ObjectRef>) -> ObjectRef {
    let mut properties = HashMap::from([
        ("enumerable".to_owned(), Value::Boolean(property.enumerable)),
        (
            "configurable".to_owned(),
            Value::Boolean(property.configurable),
        ),
    ]);
    if property.is_accessor() {
        properties.insert("get".to_owned(), property.get.unwrap_or(Value::Undefined));
        properties.insert("set".to_owned(), property.set.unwrap_or(Value::Undefined));
    } else {
        properties.insert("value".to_owned(), property.value);
        properties.insert("writable".to_owned(), Value::Boolean(property.writable));
    }
    ObjectRef::with_prototype(properties, prototype)
}
