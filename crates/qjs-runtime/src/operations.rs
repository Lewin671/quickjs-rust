use std::collections::HashMap;

use qjs_ast::{BinaryOp, UnaryOp};

use crate::{
    Property, RuntimeError, Value, has_property, is_truthy, string_object_value, to_int32,
    to_int32_number, to_js_string_with_env, to_number, to_number_with_env, to_primitive_with_env,
    to_property_key, to_uint32_number, value_prototype,
};

pub(crate) fn eval_unary(op: UnaryOp, argument: Value) -> Result<Value, RuntimeError> {
    match op {
        UnaryOp::Not => Ok(Value::Boolean(!is_truthy(&argument))),
        UnaryOp::Plus => Ok(Value::Number(to_number(argument)?)),
        UnaryOp::Minus => Ok(Value::Number(-to_number(argument)?)),
        UnaryOp::BitwiseNot => Ok(Value::Number(f64::from(!to_int32(argument)?))),
        UnaryOp::Void => Ok(Value::Undefined),
        UnaryOp::Typeof | UnaryOp::Delete => {
            unreachable!("operator requires unevaluated operand handling")
        }
    }
}

pub(crate) fn eval_binary(
    left: Value,
    op: BinaryOp,
    right: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if op == BinaryOp::In {
        return eval_in(left, right, env);
    }
    if op == BinaryOp::Instanceof {
        return eval_instanceof(left, right, env);
    }

    match op {
        BinaryOp::Eq => return Ok(Value::Boolean(abstract_eq(&left, &right)?)),
        BinaryOp::Ne => return Ok(Value::Boolean(!abstract_eq(&left, &right)?)),
        BinaryOp::StrictEq => return Ok(Value::Boolean(left == right)),
        BinaryOp::StrictNe => return Ok(Value::Boolean(left != right)),
        BinaryOp::Add => {
            let left = to_primitive_with_env(left, env)?;
            let right = to_primitive_with_env(right, env)?;
            if matches!(left, Value::String(_)) || matches!(right, Value::String(_)) {
                return Ok(Value::String(format!(
                    "{}{}",
                    to_js_string_with_env(left, env)?,
                    to_js_string_with_env(right, env)?
                )));
            }
            return Ok(Value::Number(
                to_number_with_env(left, env)? + to_number_with_env(right, env)?,
            ));
        }
        _ => {}
    }

    let left = to_number(left)?;
    let right = to_number(right)?;

    let value = match op {
        BinaryOp::Add => left + right,
        BinaryOp::Sub => left - right,
        BinaryOp::Mul => left * right,
        BinaryOp::Pow => left.powf(right),
        BinaryOp::Div => left / right,
        BinaryOp::Rem => left % right,
        BinaryOp::Shl => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) << (to_uint32_number(right) & 0x1f),
            )));
        }
        BinaryOp::Shr => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) >> (to_uint32_number(right) & 0x1f),
            )));
        }
        BinaryOp::UShr => {
            return Ok(Value::Number(f64::from(
                to_uint32_number(left) >> (to_uint32_number(right) & 0x1f),
            )));
        }
        BinaryOp::BitwiseAnd => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) & to_int32_number(right),
            )));
        }
        BinaryOp::BitwiseXor => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) ^ to_int32_number(right),
            )));
        }
        BinaryOp::BitwiseOr => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) | to_int32_number(right),
            )));
        }
        BinaryOp::Lt => return Ok(Value::Boolean(left < right)),
        BinaryOp::Le => return Ok(Value::Boolean(left <= right)),
        BinaryOp::Gt => return Ok(Value::Boolean(left > right)),
        BinaryOp::Ge => return Ok(Value::Boolean(left >= right)),
        BinaryOp::Eq
        | BinaryOp::StrictEq
        | BinaryOp::Ne
        | BinaryOp::StrictNe
        | BinaryOp::In
        | BinaryOp::Instanceof
        | BinaryOp::LogicalAnd
        | BinaryOp::LogicalOr
        | BinaryOp::NullishCoalescing => unreachable!("handled before numeric binary evaluation"),
    };
    Ok(Value::Number(value))
}

fn abstract_eq(left: &Value, right: &Value) -> Result<bool, RuntimeError> {
    match (left, right) {
        (Value::Null, Value::Undefined) | (Value::Undefined, Value::Null) => Ok(true),
        (Value::Number(_), Value::String(value)) => {
            Ok(left == &Value::Number(to_number(Value::String(value.clone()))?))
        }
        (Value::String(value), Value::Number(_)) => {
            Ok(&Value::Number(to_number(Value::String(value.clone()))?) == right)
        }
        (Value::Boolean(value), _) => {
            abstract_eq(&Value::Number(if *value { 1.0 } else { 0.0 }), right)
        }
        (_, Value::Boolean(value)) => {
            abstract_eq(left, &Value::Number(if *value { 1.0 } else { 0.0 }))
        }
        (Value::Object(object), Value::String(_) | Value::Number(_)) => {
            match string_object_value(object) {
                Some(value) => abstract_eq(&Value::String(value), right),
                None => Ok(false),
            }
        }
        (Value::String(_) | Value::Number(_), Value::Object(object)) => {
            match string_object_value(object) {
                Some(value) => abstract_eq(left, &Value::String(value)),
                None => Ok(false),
            }
        }
        _ => Ok(left == right),
    }
}

fn eval_instanceof(
    left: Value,
    right: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Function(constructor) = right else {
        return Err(RuntimeError {
            thrown: None,
            message: "right-hand side of instanceof is not callable".to_owned(),
        });
    };
    let Some(left_prototype) = value_prototype(left, env) else {
        return Ok(Value::Boolean(false));
    };
    let Some(Property {
        value: Value::Object(prototype),
        ..
    }) = constructor.properties.borrow().get("prototype").cloned()
    else {
        return Err(RuntimeError {
            thrown: None,
            message: "function prototype is not an object".to_owned(),
        });
    };
    Ok(Value::Boolean(
        left_prototype.ptr_eq(&prototype) || left_prototype.has_prototype(&prototype),
    ))
}

fn eval_in(left: Value, right: Value, env: &HashMap<String, Value>) -> Result<Value, RuntimeError> {
    let key = to_property_key(left)?;
    has_property(right, env, &key)
        .map(Value::Boolean)
        .map_err(|_| RuntimeError {
            thrown: None,
            message: "right operand of in is not an object".to_owned(),
        })
}
