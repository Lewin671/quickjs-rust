use std::collections::HashMap;

use qjs_ast::BinaryOp;

use crate::{
    Property, RuntimeError, Value, array_prototype, boolean, call_function,
    function_delete_own_property, function_intrinsic_prototype, function_own_property_keys,
    inherited_string_prototype_property, number, object_prototype, property_value, string,
    string_prototype, to_length,
};

pub(super) fn get_property(
    object: Value,
    key: &str,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match object {
        Value::Array(elements) if key == "length" => Ok(Value::Number(elements.len() as f64)),
        Value::Array(elements) => property_value(Value::Array(elements), key, env),
        Value::Function(function) => property_value(Value::Function(function), key, env),
        Value::String(value) if key == "length" => Ok(Value::Number(value.chars().count() as f64)),
        Value::String(value) => Ok(string::string_property(&value, key)
            .or_else(|| inherited_string_prototype_property(env, key))
            .unwrap_or(Value::Undefined)),
        Value::Boolean(_) => {
            Ok(boolean::inherited_boolean_prototype_property(env, key).unwrap_or(Value::Undefined))
        }
        Value::Number(_) => {
            Ok(number::inherited_number_prototype_property(env, key).unwrap_or(Value::Undefined))
        }
        Value::Object(_) => property_value(object, key, env),
        _ => Err(RuntimeError {
            thrown: None,
            message: format!("unsupported property `{key}`"),
        }),
    }
}

pub(super) fn set_property(
    object: Value,
    key: String,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    match object {
        Value::Object(object) => {
            let property = object.property(&key);
            if apply_property_setter(
                property.clone(),
                Value::Object(object.clone()),
                value.clone(),
                env,
            )? {
                return Ok(true);
            }
            if property.is_some_and(|property| property.is_accessor() || !property.writable) {
                return Ok(false);
            }
            if object.own_property(&key).is_none() && !object.is_extensible() {
                return Ok(false);
            }
            object.set(key, value);
            Ok(true)
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
                return Ok(true);
            }
            if property.is_some_and(|property| property.is_accessor() || !property.writable) {
                return Ok(false);
            }
            if !function.properties.borrow().contains_key(&key) && !function.is_extensible() {
                return Ok(false);
            }
            function.set_property(key, value);
            Ok(true)
        }
        Value::Array(elements) => {
            if key == "length" {
                Ok(elements.try_set_len(to_length(value)?))
            } else {
                let property = elements.property(&key).or_else(|| {
                    elements
                        .prototype_override()
                        .unwrap_or_else(|| array_prototype(env))
                        .and_then(|prototype| prototype.property(&key))
                });
                if apply_property_setter(
                    property.clone(),
                    Value::Array(elements.clone()),
                    value.clone(),
                    env,
                )? {
                    return Ok(true);
                }
                if property.is_some_and(|property| property.is_accessor() || !property.writable) {
                    return Ok(false);
                }
                Ok(match key.parse::<usize>() {
                    Ok(index) => elements.try_set(index, value),
                    Err(_) => elements.try_set_property(key, value),
                })
            }
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "member assignment target is not an object".to_owned(),
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
    if let Some(setter) = property.set {
        call_function(setter, receiver, vec![value], env, false)?;
        return Ok(true);
    }
    Ok(false)
}

pub(super) fn delete_property(object: Value, key: &str) -> Result<Value, RuntimeError> {
    match object {
        Value::Object(object) => Ok(Value::Boolean(object.delete_own_property(key))),
        Value::Array(elements) => Ok(Value::Boolean(match key.parse::<usize>() {
            Ok(index) => elements.delete_index(index),
            Err(_) => elements.delete_property(key),
        })),
        Value::Function(function) => {
            Ok(Value::Boolean(function_delete_own_property(&function, key)))
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "delete target is not an object".to_owned(),
        }),
    }
}

pub(super) fn enumerable_keys(
    value: Value,
    env: &HashMap<String, Value>,
) -> Result<Vec<Value>, RuntimeError> {
    let keys = match value {
        Value::Object(object) => {
            enumerable_object_keys(object.own_property_keys(), object.prototype())
        }
        Value::Array(elements) => {
            let mut keys: Vec<_> = (0..elements.len())
                .filter(|index| elements.has_index(*index))
                .map(|index| index.to_string())
                .collect();
            keys.extend(elements.property_keys());
            enumerable_object_keys(
                keys,
                elements
                    .prototype_override()
                    .unwrap_or_else(|| array_prototype(env)),
            )
        }
        Value::Function(function) => {
            let keys = function_own_property_keys(&function);
            enumerable_object_keys(
                keys,
                function
                    .internal_prototype_override()
                    .unwrap_or_else(|| function_intrinsic_prototype(env)),
            )
        }
        Value::String(value) => enumerable_object_keys(
            crate::string::string_own_property_keys(&value),
            string_prototype(env),
        ),
        Value::Number(_) | Value::Boolean(_) => {
            enumerable_object_keys(Vec::new(), object_prototype(env))
        }
        Value::Null | Value::Undefined => Vec::new(),
    };
    Ok(keys.into_iter().map(Value::String).collect())
}

fn enumerable_object_keys(
    mut keys: Vec<String>,
    mut prototype: Option<crate::ObjectRef>,
) -> Vec<String> {
    while let Some(object) = prototype {
        for key in object.own_property_keys() {
            if !keys.iter().any(|existing| existing == &key) {
                keys.push(key);
            }
        }
        prototype = object.prototype();
    }
    keys
}

pub(super) fn fast_number_binary(left: &Value, op: BinaryOp, right: &Value) -> Option<Value> {
    let (Value::Number(left), Value::Number(right)) = (left, right) else {
        return None;
    };
    let value = match op {
        BinaryOp::Add => Value::Number(left + right),
        BinaryOp::Sub => Value::Number(left - right),
        BinaryOp::Mul => Value::Number(left * right),
        BinaryOp::Div => Value::Number(left / right),
        BinaryOp::Rem => Value::Number(left % right),
        BinaryOp::Pow => Value::Number(left.powf(*right)),
        BinaryOp::Lt => Value::Boolean(left < right),
        BinaryOp::Le => Value::Boolean(left <= right),
        BinaryOp::Gt => Value::Boolean(left > right),
        BinaryOp::Ge => Value::Boolean(left >= right),
        BinaryOp::StrictEq => Value::Boolean(left == right),
        BinaryOp::StrictNe => Value::Boolean(left != right),
        _ => return None,
    };
    Some(value)
}
