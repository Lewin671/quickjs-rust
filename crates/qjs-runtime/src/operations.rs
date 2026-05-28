use std::collections::HashMap;

use qjs_ast::{BinaryOp, UnaryOp};

use crate::{
    Property, RuntimeError, Value, is_truthy, to_int32, to_int32_number, to_js_string, to_number,
    to_property_key, to_uint32_number, value_prototype,
};

pub(super) fn eval_unary(op: UnaryOp, argument: Value) -> Result<Value, RuntimeError> {
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

pub(super) fn eval_binary(
    left: Value,
    op: BinaryOp,
    right: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if op == BinaryOp::In {
        return eval_in(left, right);
    }
    if op == BinaryOp::Instanceof {
        return eval_instanceof(left, right, env);
    }

    match op {
        BinaryOp::Eq | BinaryOp::StrictEq => return Ok(Value::Boolean(left == right)),
        BinaryOp::Ne | BinaryOp::StrictNe => return Ok(Value::Boolean(left != right)),
        BinaryOp::Add if matches!(left, Value::String(_)) || matches!(right, Value::String(_)) => {
            return Ok(Value::String(format!(
                "{}{}",
                to_js_string(left)?,
                to_js_string(right)?
            )));
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

fn eval_instanceof(
    left: Value,
    right: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Function(constructor) = right else {
        return Err(RuntimeError {
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
            message: "function prototype is not an object".to_owned(),
        });
    };
    Ok(Value::Boolean(
        left_prototype.ptr_eq(&prototype) || left_prototype.has_prototype(&prototype),
    ))
}

fn eval_in(left: Value, right: Value) -> Result<Value, RuntimeError> {
    let key = to_property_key(left)?;
    match right {
        Value::Object(object) => Ok(Value::Boolean(object.contains_property(&key))),
        Value::Array(elements) => {
            let index = key.parse::<usize>().ok();
            Ok(Value::Boolean(
                index.is_some_and(|index| index < elements.len()) || key == "length",
            ))
        }
        _ => Err(RuntimeError {
            message: "right operand of in is not an object".to_owned(),
        }),
    }
}
