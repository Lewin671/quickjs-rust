use std::collections::HashMap;

use qjs_ast::{BinaryOp, Expr, Literal, ObjectPropertyKey, UnaryOp};

use crate::{
    ArrayRef, Function, GLOBAL_THIS_BINDING, ObjectRef, RuntimeError, Value, call_function,
    constructor_prototype, is_truthy, object_prototype, operations, to_property_key,
};

mod assignment;
mod member;

pub(crate) use assignment::assign_target;

use assignment::{eval_assignment, eval_update};
use member::{eval_delete, eval_member};

fn eval_call(
    callee: &Expr,
    arguments: &[Expr],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let (callee, this_value) = match callee {
        Expr::Member {
            object, property, ..
        } => {
            let object = eval_expr(object, env)?;
            let callee = eval_member(object.clone(), property, env)?;
            (callee, object)
        }
        _ => {
            let callee = eval_expr(callee, env)?;
            let this_value = env
                .get(GLOBAL_THIS_BINDING)
                .cloned()
                .unwrap_or(Value::Undefined);
            (callee, this_value)
        }
    };

    let argument_values = eval_arguments(arguments, env)?;
    call_function(callee, this_value, argument_values, env, false)
}

fn eval_new(
    callee: &Expr,
    arguments: &[Expr],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let callee = eval_expr(callee, env)?;
    let Value::Function(function) = &callee else {
        return Err(RuntimeError {
            message: "value is not a constructor".to_owned(),
        });
    };
    if !function.constructable {
        return Err(RuntimeError {
            message: "value is not a constructor".to_owned(),
        });
    }
    let argument_values = eval_arguments(arguments, env)?;
    let prototype = constructor_prototype(&callee);
    let this_value = Value::Object(ObjectRef::with_prototype(HashMap::new(), prototype));
    let result = call_function(callee, this_value.clone(), argument_values, env, true)?;
    match result {
        Value::Array(_) | Value::Function(_) | Value::Object(_) => Ok(result),
        _ => Ok(this_value),
    }
}

fn eval_arguments(
    arguments: &[Expr],
    env: &mut HashMap<String, Value>,
) -> Result<Vec<Value>, RuntimeError> {
    let mut argument_values = Vec::with_capacity(arguments.len());
    for argument in arguments {
        argument_values.push(eval_expr(argument, env)?);
    }
    Ok(argument_values)
}

pub(crate) fn eval_expr(
    expr: &Expr,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match expr {
        Expr::Literal(literal) => eval_literal(literal),
        Expr::Array { elements, .. } => {
            let mut values = Vec::with_capacity(elements.len());
            for element in elements {
                values.push(eval_expr(element, env)?);
            }
            Ok(Value::Array(ArrayRef::new(values)))
        }
        Expr::Object { properties, .. } => {
            let mut values = HashMap::new();
            for property in properties {
                let key = match &property.key {
                    ObjectPropertyKey::Literal(key) => key.clone(),
                    ObjectPropertyKey::Computed(expr) => to_property_key(eval_expr(expr, env)?)?,
                };
                values.insert(key, eval_expr(&property.value, env)?);
            }
            Ok(Value::Object(ObjectRef::with_prototype(
                values,
                object_prototype(env),
            )))
        }
        Expr::Function {
            name,
            params,
            body,
            constructable,
            ..
        } => Ok(Value::Function(Function::new_user_with_constructable(
            name.clone(),
            params.clone(),
            body.clone(),
            env.clone(),
            *constructable,
        ))),
        Expr::Sequence { expressions, .. } => {
            let mut last = Value::Undefined;
            for expression in expressions {
                last = eval_expr(expression, env)?;
            }
            Ok(last)
        }
        Expr::This { .. } => env.get("this").cloned().ok_or_else(|| RuntimeError {
            message: "missing this binding".to_owned(),
        }),
        Expr::Identifier { name, .. } => env.get(name).cloned().ok_or_else(|| RuntimeError {
            message: format!("undefined identifier `{name}`"),
        }),
        Expr::Unary {
            op: UnaryOp::Typeof,
            argument,
            ..
        } => eval_typeof(argument, env),
        Expr::Unary {
            op: UnaryOp::Delete,
            argument,
            ..
        } => eval_delete(argument, env),
        Expr::Unary { op, argument, .. } => {
            let argument = eval_expr(argument, env)?;
            operations::eval_unary(*op, argument)
        }
        Expr::Assignment {
            target, op, value, ..
        } => eval_assignment(target, *op, value, env),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            let test = eval_expr(test, env)?;
            if is_truthy(&test) {
                eval_expr(consequent, env)
            } else {
                eval_expr(alternate, env)
            }
        }
        Expr::Update {
            target, op, prefix, ..
        } => eval_update(target, *op, *prefix, env),
        Expr::Call {
            callee, arguments, ..
        } => eval_call(callee, arguments, env),
        Expr::New {
            callee, arguments, ..
        } => eval_new(callee, arguments, env),
        Expr::Member {
            object, property, ..
        } => {
            let object = eval_expr(object, env)?;
            eval_member(object, property, env)
        }
        Expr::Binary {
            left, op, right, ..
        } if *op == BinaryOp::LogicalAnd => {
            let left = eval_expr(left, env)?;
            if is_truthy(&left) {
                eval_expr(right, env)
            } else {
                Ok(left)
            }
        }
        Expr::Binary {
            left, op, right, ..
        } if *op == BinaryOp::LogicalOr => {
            let left = eval_expr(left, env)?;
            if is_truthy(&left) {
                Ok(left)
            } else {
                eval_expr(right, env)
            }
        }
        Expr::Binary {
            left, op, right, ..
        } if *op == BinaryOp::NullishCoalescing => {
            let left = eval_expr(left, env)?;
            if matches!(left, Value::Null | Value::Undefined) {
                eval_expr(right, env)
            } else {
                Ok(left)
            }
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            let left = eval_expr(left, env)?;
            let right = eval_expr(right, env)?;
            operations::eval_binary(left, *op, right, env)
        }
    }
}

fn eval_literal(literal: &Literal) -> Result<Value, RuntimeError> {
    match literal {
        Literal::Number { raw, .. } => parse_number_literal(raw).map(Value::Number),
        Literal::String { value, .. } => Ok(Value::String(value.clone())),
        Literal::Boolean { value, .. } => Ok(Value::Boolean(*value)),
        Literal::Null { .. } => Ok(Value::Null),
    }
}

fn parse_number_literal(raw: &str) -> Result<f64, RuntimeError> {
    if let Some(digits) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        Ok(parse_radix_number(digits, 16))
    } else if let Some(digits) = raw.strip_prefix("0b").or_else(|| raw.strip_prefix("0B")) {
        Ok(parse_radix_number(digits, 2))
    } else if let Some(digits) = raw.strip_prefix("0o").or_else(|| raw.strip_prefix("0O")) {
        Ok(parse_radix_number(digits, 8))
    } else {
        raw.parse::<f64>().map_err(|_| RuntimeError {
            message: format!("invalid number literal `{raw}`"),
        })
    }
}

fn parse_radix_number(digits: &str, radix: u32) -> f64 {
    digits.chars().fold(0.0, |value, digit| {
        value * f64::from(radix) + f64::from(digit.to_digit(radix).unwrap_or(0))
    })
}

fn eval_typeof(expr: &Expr, env: &mut HashMap<String, Value>) -> Result<Value, RuntimeError> {
    let value = match expr {
        Expr::Identifier { name, .. } => env.get(name).cloned().unwrap_or(Value::Undefined),
        _ => eval_expr(expr, env)?,
    };
    let type_name = match value {
        Value::Undefined => "undefined",
        Value::Boolean(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Function(_) => "function",
        Value::Null | Value::Array(_) | Value::Object(_) => "object",
    };
    Ok(Value::String(type_name.to_owned()))
}
