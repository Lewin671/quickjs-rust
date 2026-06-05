use std::collections::HashMap;

use crate::{Property, RuntimeError, Value, call_function};

mod array;
mod function;
mod key;
mod prototype;

pub(crate) use array::{
    array_has_own_property, array_own_property_descriptor, array_own_property_keys,
    array_own_property_names,
};
pub(crate) use function::{
    function_delete_own_property, function_own_property_descriptor, function_own_property_keys,
    function_own_property_names,
};
pub(crate) use key::to_property_key;
pub(crate) use prototype::{
    array_prototype, array_prototype_property, constructor_prototype, function_intrinsic_prototype,
    function_prototype, function_prototype_property, inherited_object_prototype_property,
    inherited_primitive_prototype_descriptor, inherited_string_prototype_property,
    object_prototype, string_prototype, value_prototype,
};

pub(crate) fn has_property(
    value: Value,
    env: &HashMap<String, Value>,
    key: &str,
) -> Result<bool, RuntimeError> {
    match value {
        Value::Object(object) => Ok(object.contains_property(key)),
        Value::Map(map) => Ok(map.object().contains_property(key)),
        Value::Set(set) => Ok(set.object().contains_property(key)),
        Value::Array(elements) => Ok(array_has_own_property(&elements, key)
            || array_prototype_property(&elements, env, key).is_some()),
        Value::Function(function) => Ok(function_own_property_descriptor(&function, key).is_some()
            || function_prototype_property(&function, env, key).is_some()),
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "property target must be an object".to_owned(),
        }),
    }
}

pub(crate) fn property_value(
    receiver: Value,
    key: &str,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match receiver.clone() {
        Value::Object(object) => property_descriptor_value(object.property(key), receiver, env),
        Value::Map(map) => property_descriptor_value(map.object().property(key), receiver, env),
        Value::Set(set) => property_descriptor_value(set.object().property(key), receiver, env),
        Value::Function(function) => property_descriptor_value(
            function_own_property_descriptor(&function, key).or_else(|| {
                function
                    .internal_prototype_override()
                    .unwrap_or_else(|| function_intrinsic_prototype(env))
                    .and_then(|prototype| prototype.property(key))
            }),
            receiver,
            env,
        ),
        Value::Array(elements) => {
            if key == "length" {
                Ok(Value::Number(elements.len() as f64))
            } else {
                let descriptor = key
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| elements.get(index).map(Property::enumerable))
                    .or_else(|| elements.property(key))
                    .or_else(|| {
                        elements
                            .prototype_override()
                            .unwrap_or_else(|| array_prototype(env))
                            .and_then(|prototype| prototype.property(key))
                    });
                property_descriptor_value(descriptor, receiver, env)
            }
        }
        Value::String(value) => {
            if key == "length" {
                Ok(Value::Number(value.chars().count() as f64))
            } else {
                let descriptor = crate::string::string_property(&value, key)
                    .map(|value| Property::data(value, true, false, false))
                    .or_else(|| inherited_primitive_prototype_descriptor(env, "String", key));
                property_descriptor_value(descriptor, receiver, env)
            }
        }
        Value::Boolean(_) => property_descriptor_value(
            inherited_primitive_prototype_descriptor(env, "Boolean", key),
            receiver,
            env,
        ),
        Value::Number(_) => property_descriptor_value(
            inherited_primitive_prototype_descriptor(env, "Number", key),
            receiver,
            env,
        ),
        Value::Null | Value::Undefined => Ok(Value::Undefined),
    }
}

fn property_descriptor_value(
    property: Option<Property>,
    receiver: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Some(property) = property else {
        return Ok(Value::Undefined);
    };
    if let Some(getter) = property.get {
        return call_function(getter, receiver, Vec::new(), env, false);
    }
    Ok(property.value)
}
