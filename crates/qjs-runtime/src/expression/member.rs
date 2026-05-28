use std::collections::HashMap;

use qjs_ast::{Expr, MemberProperty};

use crate::{
    Property, RuntimeError, Value, boolean, inherited_array_prototype_property,
    inherited_function_prototype_property, inherited_string_prototype_property, number, string,
    to_array_index, to_length, to_property_key,
};

use super::eval_expr;

pub(super) fn eval_member(
    object: Value,
    property: &MemberProperty,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match (object, property) {
        (Value::Array(elements), MemberProperty::Named(name)) if name == "length" => {
            Ok(Value::Number(elements.len() as f64))
        }
        (Value::Array(_), MemberProperty::Named(name)) => {
            Ok(inherited_array_prototype_property(env, name).unwrap_or(Value::Undefined))
        }
        (Value::Function(function), MemberProperty::Named(name)) if name == "length" => {
            Ok(Value::Number(function.params.len() as f64))
        }
        (Value::Function(function), property) => {
            let key = property_key(property, env)?;
            Ok(function
                .properties
                .borrow()
                .get(&key)
                .map(|property| property.value.clone())
                .or_else(|| inherited_function_prototype_property(env, &key))
                .unwrap_or(Value::Undefined))
        }
        (Value::String(value), MemberProperty::Named(name)) if name == "length" => {
            Ok(Value::Number(value.chars().count() as f64))
        }
        (Value::String(value), property) => {
            let key = property_key(property, env)?;
            Ok(string::string_property(&value, &key)
                .or_else(|| inherited_string_prototype_property(env, &key))
                .unwrap_or(Value::Undefined))
        }
        (Value::Boolean(_), MemberProperty::Named(name)) => Ok(
            boolean::inherited_boolean_prototype_property(env, name).unwrap_or(Value::Undefined),
        ),
        (Value::Number(_), MemberProperty::Named(name)) => {
            Ok(number::inherited_number_prototype_property(env, name).unwrap_or(Value::Undefined))
        }
        (Value::Array(elements), MemberProperty::Computed(index)) => {
            let index = eval_expr(index, env)?;
            let index = to_array_index(index)?;
            Ok(elements.get(index).unwrap_or(Value::Undefined))
        }
        (Value::Object(object), property) => {
            let key = property_key(property, env)?;
            Ok(object.get(&key).unwrap_or(Value::Undefined))
        }
        (_, MemberProperty::Named(name)) => Err(RuntimeError {
            message: format!("unsupported property `{name}`"),
        }),
        (_, MemberProperty::Computed(_)) => Err(RuntimeError {
            message: "unsupported computed member access".to_owned(),
        }),
    }
}

pub(super) fn assign_member(
    object: Value,
    property: &MemberProperty,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let key = property_key(property, env)?;
    match object {
        Value::Object(object) => {
            object.set(key, value);
            Ok(())
        }
        Value::Function(function) => {
            function
                .properties
                .borrow_mut()
                .insert(key, Property::enumerable(value));
            Ok(())
        }
        Value::Array(elements) => {
            if key == "length" {
                elements.set_len(to_length(value)?);
            } else {
                let index = key.parse::<usize>().map_err(|_| RuntimeError {
                    message: "array property assignment requires an array index".to_owned(),
                })?;
                elements.set(index, value);
            }
            Ok(())
        }
        _ => Err(RuntimeError {
            message: "member assignment target is not an object".to_owned(),
        }),
    }
}

fn property_key(
    property: &MemberProperty,
    env: &mut HashMap<String, Value>,
) -> Result<String, RuntimeError> {
    match property {
        MemberProperty::Named(name) => Ok(name.clone()),
        MemberProperty::Computed(expr) => to_property_key(eval_expr(expr, env)?),
    }
}

pub(super) fn eval_delete(
    expr: &Expr,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Expr::Member {
        object, property, ..
    } = expr
    else {
        return Ok(Value::Boolean(true));
    };

    let object = eval_expr(object, env)?;
    match object {
        Value::Object(object) => {
            let key = property_key(property, env)?;
            object.delete_own_property(&key);
            Ok(Value::Boolean(true))
        }
        Value::Array(_) => Ok(Value::Boolean(true)),
        _ => Err(RuntimeError {
            message: "delete target is not an object".to_owned(),
        }),
    }
}
