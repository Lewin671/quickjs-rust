use std::collections::HashMap;

use qjs_ast::BinaryOp;

use crate::{
    RuntimeError, Value, array_prototype_property, boolean, function_prototype_property,
    inherited_string_prototype_property, number, property_value, string, to_length,
};

pub(super) fn get_property(
    object: Value,
    key: &str,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match object {
        Value::Array(elements) if key == "length" => Ok(Value::Number(elements.len() as f64)),
        Value::Array(elements) => Ok(key
            .parse::<usize>()
            .ok()
            .and_then(|index| elements.get(index))
            .or_else(|| array_prototype_property(&elements, env, key))
            .unwrap_or(Value::Undefined)),
        Value::Function(function) if key == "length" => {
            Ok(Value::Number(function.params.len() as f64))
        }
        Value::Function(function) => Ok(function
            .properties
            .borrow()
            .get(key)
            .map(|property| property.value.clone())
            .or_else(|| function_prototype_property(&function, env, key))
            .unwrap_or(Value::Undefined)),
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

pub(super) fn set_property(object: Value, key: String, value: Value) -> Result<(), RuntimeError> {
    match object {
        Value::Object(object) => {
            object.set(key, value);
            Ok(())
        }
        Value::Function(function) => {
            function.set_property(key, value);
            Ok(())
        }
        Value::Array(elements) => {
            if key == "length" {
                elements.set_len(to_length(value)?);
            } else {
                let index = key.parse::<usize>().map_err(|_| RuntimeError {
                    thrown: None,
                    message: "array property assignment requires an array index".to_owned(),
                })?;
                elements.set(index, value);
            }
            Ok(())
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "member assignment target is not an object".to_owned(),
        }),
    }
}

pub(super) fn delete_property(object: Value, key: &str) -> Result<Value, RuntimeError> {
    match object {
        Value::Object(object) => Ok(Value::Boolean(object.delete_own_property(key))),
        Value::Array(_) => Ok(Value::Boolean(true)),
        _ => Err(RuntimeError {
            thrown: None,
            message: "delete target is not an object".to_owned(),
        }),
    }
}

pub(super) fn enumerable_keys(value: Value) -> Result<Vec<Value>, RuntimeError> {
    let keys = match value {
        Value::Object(object) => object.own_property_keys(),
        Value::Array(elements) => (0..elements.len()).map(|index| index.to_string()).collect(),
        Value::Null | Value::Undefined => Vec::new(),
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "for-in target is not enumerable".to_owned(),
            });
        }
    };
    Ok(keys.into_iter().map(Value::String).collect())
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
