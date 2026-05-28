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

    define_property_on_value(target.clone(), key, descriptor)?;
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
            message: "property descriptors must be an object".to_owned(),
        });
    }

    for (key, descriptor_value) in enumerable_property_entries(descriptors)? {
        let descriptor = to_property_descriptor(descriptor_value)?;
        define_property_on_value(target.clone(), key, descriptor)?;
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

fn define_property_on_value(
    target: Value,
    key: String,
    descriptor: Property,
) -> Result<(), RuntimeError> {
    match &target {
        Value::Object(object) => {
            if !object.has_own_property(&key) && !object.is_extensible() {
                return Err(RuntimeError {
                    message: "object is not extensible".to_owned(),
                });
            }
            if object
                .own_property(&key)
                .is_some_and(|property| !property.configurable && descriptor.configurable)
            {
                return Err(RuntimeError {
                    message: "property is not configurable".to_owned(),
                });
            }
            object.define_property(key, descriptor);
            Ok(())
        }
        Value::Function(function) => {
            let existing = function_own_property_descriptor(function, &key);
            if existing.is_none() && !function.is_extensible() {
                return Err(RuntimeError {
                    message: "function is not extensible".to_owned(),
                });
            }
            if existing.is_some_and(|property| !property.configurable && descriptor.configurable) {
                return Err(RuntimeError {
                    message: "property is not configurable".to_owned(),
                });
            }
            function.properties.borrow_mut().insert(key, descriptor);
            Ok(())
        }
        _ => ensure_define_property_target(&target),
    }
}

fn ensure_define_property_target(target: &Value) -> Result<(), RuntimeError> {
    match target {
        Value::Object(_) | Value::Function(_) => Ok(()),
        Value::Array(_) | Value::String(_) | Value::Number(_) | Value::Boolean(_) => {
            Err(RuntimeError {
                message: "Object.defineProperty primitive targets are not implemented".to_owned(),
            })
        }
        Value::Null | Value::Undefined => Err(RuntimeError {
            message: "Object.defineProperty target must be an object".to_owned(),
        }),
    }
}

fn to_property_descriptor(value: Value) -> Result<Property, RuntimeError> {
    let Value::Object(descriptor) = value else {
        return Err(RuntimeError {
            message: "property descriptor must be an object".to_owned(),
        });
    };

    if descriptor.contains_property("get") || descriptor.contains_property("set") {
        return Err(RuntimeError {
            message: "accessor property descriptors are not implemented".to_owned(),
        });
    }

    Ok(Property {
        value: descriptor.get("value").unwrap_or(Value::Undefined),
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

fn property_descriptor_object(property: Property, prototype: Option<ObjectRef>) -> ObjectRef {
    ObjectRef::with_prototype(
        HashMap::from([
            ("value".to_owned(), property.value),
            ("writable".to_owned(), Value::Boolean(property.writable)),
            ("enumerable".to_owned(), Value::Boolean(property.enumerable)),
            (
                "configurable".to_owned(),
                Value::Boolean(property.configurable),
            ),
        ]),
        prototype,
    )
}
