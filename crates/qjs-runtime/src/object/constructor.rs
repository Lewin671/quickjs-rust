use std::collections::HashMap;

use crate::{
    Function, ObjectRef, Property, RuntimeError, Value, boolean::BOOLEAN_DATA_PROPERTY,
    call_function, function_intrinsic_prototype, function_prototype, number::NUMBER_DATA_PROPERTY,
    string::STRING_DATA_PROPERTY,
};

use super::descriptor::native_object_define_properties;
use super::enumeration::enumerable_property_entries;

pub(crate) fn native_object_assign(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let target = match target {
        Value::Array(_) | Value::Object(_) | Value::Function(_) => target,
        Value::Null | Value::Undefined => {
            return Err(RuntimeError {
                thrown: None,
                message: "Object.assign target must not be null or undefined".to_owned(),
            });
        }
        Value::String(_) | Value::Number(_) | Value::Boolean(_) => boxed_primitive(target, env)
            .ok_or_else(|| RuntimeError {
                thrown: None,
                message: "Object.assign target could not be boxed".to_owned(),
            })?,
    };

    for source in argument_values.iter().skip(1).cloned() {
        if matches!(source, Value::Null | Value::Undefined) {
            continue;
        }
        for (key, value) in enumerable_property_entries(source, env)? {
            set_property(target.clone(), key, value, env)?;
        }
    }
    Ok(target)
}

pub(crate) fn native_object(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match argument_values.first() {
        Some(Value::Array(_) | Value::Function(_) | Value::Object(_)) => {
            Ok(argument_values[0].clone())
        }
        Some(Value::Boolean(value)) => Ok(boxed_boolean(*value, env)),
        Some(Value::Number(value)) => Ok(boxed_number(*value, env)),
        Some(Value::String(value)) => Ok(boxed_string(value, env)),
        _ if is_construct => Ok(this_value),
        _ => Ok(Value::Object(ObjectRef::with_prototype(
            HashMap::new(),
            function_prototype(function),
        ))),
    }
}

pub(crate) fn boxed_primitive(value: Value, env: &HashMap<String, Value>) -> Option<Value> {
    match value {
        Value::Boolean(value) => Some(boxed_boolean(value, env)),
        Value::Number(value) => Some(boxed_number(value, env)),
        Value::String(value) => Some(boxed_string(&value, env)),
        _ => None,
    }
}

fn boxed_boolean(value: bool, env: &HashMap<String, Value>) -> Value {
    let object = ObjectRef::with_prototype(HashMap::new(), constructor_prototype("Boolean", env));
    object.define_non_enumerable(BOOLEAN_DATA_PROPERTY.to_owned(), Value::Boolean(value));
    Value::Object(object)
}

fn boxed_number(value: f64, env: &HashMap<String, Value>) -> Value {
    let object = ObjectRef::with_prototype(HashMap::new(), constructor_prototype("Number", env));
    object.define_non_enumerable(NUMBER_DATA_PROPERTY.to_owned(), Value::Number(value));
    Value::Object(object)
}

fn boxed_string(value: &str, env: &HashMap<String, Value>) -> Value {
    let object = ObjectRef::with_prototype(HashMap::new(), constructor_prototype("String", env));
    object.define_non_enumerable(
        STRING_DATA_PROPERTY.to_owned(),
        Value::String(value.to_owned()),
    );
    object.define_property(
        "length".to_owned(),
        Property::data(
            Value::Number(value.chars().count() as f64),
            false,
            false,
            false,
        ),
    );
    for (index, character) in value.chars().enumerate() {
        object.define_property(
            index.to_string(),
            Property::data(Value::String(character.to_string()), true, false, false),
        );
    }
    Value::Object(object)
}

fn constructor_prototype(name: &str, env: &HashMap<String, Value>) -> Option<ObjectRef> {
    let Some(Value::Function(function)) = env.get(name) else {
        return None;
    };
    function_prototype(function)
}

pub(crate) fn native_object_create(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let object = match argument_values.first() {
        Some(Value::Object(prototype)) => Value::Object(ObjectRef::with_prototype(
            HashMap::new(),
            Some(prototype.clone()),
        )),
        Some(Value::Null) => Value::Object(ObjectRef::new(HashMap::new())),
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Object.create prototype must be an object or null".to_owned(),
            });
        }
    };

    if !matches!(argument_values.get(1), None | Some(Value::Undefined)) {
        native_object_define_properties(
            &[
                object.clone(),
                argument_values.get(1).cloned().unwrap_or(Value::Undefined),
            ],
            env,
        )?;
    }
    Ok(object)
}

pub(crate) fn native_object_is(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let left = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let right = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(left.same_value(&right)))
}

fn set_property(
    target: Value,
    key: String,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    match target {
        Value::Object(object) => {
            let property = object.property(&key);
            if apply_property_setter(
                property.clone(),
                Value::Object(object.clone()),
                value.clone(),
                env,
            )? {
                return Ok(());
            }
            if property.is_some_and(|property| property.is_accessor() || !property.writable)
                || (object.own_property(&key).is_none() && !object.is_extensible())
            {
                return Err(assignment_type_error());
            }
            object.set(key, value);
            Ok(())
        }
        Value::Function(function) => {
            let property = function.properties.borrow().get(&key).cloned().or_else(|| {
                function
                    .internal_prototype_override()
                    .unwrap_or_else(|| function_intrinsic_prototype(env))
                    .and_then(|prototype| prototype.property(&key))
            });
            if apply_property_setter(
                property.clone(),
                Value::Function(function.clone()),
                value.clone(),
                env,
            )? {
                return Ok(());
            }
            if property.is_some_and(|property| property.is_accessor() || !property.writable)
                || (!function.properties.borrow().contains_key(&key) && !function.is_extensible())
            {
                return Err(assignment_type_error());
            }
            function.set_property(key, value);
            Ok(())
        }
        Value::Array(elements) => {
            if key == "length" {
                if elements.try_set_len(crate::to_length(value)?) {
                    return Ok(());
                }
            } else if let Ok(index) = key.parse::<usize>() {
                if elements.try_set(index, value) {
                    return Ok(());
                }
            } else if elements.try_set_property(key, value) {
                return Ok(());
            }
            Err(assignment_type_error())
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "property target is not mutable".to_owned(),
        }),
    }
}

fn apply_property_setter(
    property: Option<Property>,
    receiver: Value,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    let Some(property) = property else {
        return Ok(false);
    };
    let Some(setter) = property.set else {
        return Ok(false);
    };
    call_function(setter, receiver, vec![value], env, false)?;
    Ok(true)
}

fn assignment_type_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: cannot assign to property".to_owned(),
    }
}
