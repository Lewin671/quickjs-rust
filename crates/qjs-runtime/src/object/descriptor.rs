use std::collections::HashMap;

use crate::{
    ObjectRef, Property, RuntimeError, Value, function_own_property_descriptor, is_truthy,
    object_prototype, to_property_key,
};

use super::enumeration::{enumerable_property_entries, own_property_names};

pub(crate) fn native_object_define_property(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let key = to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    let descriptor =
        to_property_descriptor(argument_values.get(2).cloned().unwrap_or(Value::Undefined))?;

    if !define_property_on_value(target.clone(), key, descriptor)? {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.defineProperty failed".to_owned(),
        });
    }
    Ok(target)
}

pub(crate) fn native_object_define_properties(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_define_property_target(&target)?;

    let descriptors = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if !matches!(descriptors, Value::Object(_) | Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "property descriptors must be an object".to_owned(),
        });
    }

    for (key, descriptor_value) in enumerable_property_entries(descriptors)? {
        let descriptor = to_property_descriptor(descriptor_value)?;
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
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let key = to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    let Some(property) = own_property_descriptor(target, &key)? else {
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
    match value {
        Value::Object(object) => Ok(object.own_property(key)),
        Value::Function(function) => Ok(function_own_property_descriptor(&function, key)),
        Value::Array(elements) => Ok(crate::array_own_property_descriptor(&elements, key)),
        Value::String(value) => Ok(crate::string::string_own_property_descriptor(&value, key)),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Ok(None),
    }
}

pub(crate) fn define_property_on_value(
    target: Value,
    key: String,
    descriptor: Property,
) -> Result<bool, RuntimeError> {
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
                elements.set_len(crate::to_length(descriptor.value)?);
            } else if !descriptor.is_accessor()
                && let Ok(index) = key.parse::<usize>()
            {
                elements.set(index, descriptor.value);
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
        Value::Object(_) | Value::Function(_) | Value::Array(_) => Ok(()),
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

pub(crate) fn to_property_descriptor(value: Value) -> Result<Property, RuntimeError> {
    let Value::Object(descriptor) = value else {
        return Err(RuntimeError {
            thrown: None,
            message: "property descriptor must be an object".to_owned(),
        });
    };

    let has_get = descriptor.contains_property("get");
    let has_set = descriptor.contains_property("set");
    if has_get || has_set {
        if descriptor.contains_property("value") || descriptor.contains_property("writable") {
            return Err(RuntimeError {
                thrown: None,
                message: "property descriptor cannot mix accessor and data fields".to_owned(),
            });
        }
        return Ok(Property::accessor(
            accessor_function(descriptor.get("get").unwrap_or(Value::Undefined), "get")?,
            accessor_function(descriptor.get("set").unwrap_or(Value::Undefined), "set")?,
            descriptor
                .get("enumerable")
                .is_some_and(|value| is_truthy(&value)),
            descriptor
                .get("configurable")
                .is_some_and(|value| is_truthy(&value)),
        ));
    }

    Ok(Property {
        value: descriptor.get("value").unwrap_or(Value::Undefined),
        get: None,
        set: None,
        writable: descriptor
            .get("writable")
            .is_some_and(|value| is_truthy(&value)),
        enumerable: descriptor
            .get("enumerable")
            .is_some_and(|value| is_truthy(&value)),
        configurable: descriptor
            .get("configurable")
            .is_some_and(|value| is_truthy(&value)),
    })
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
