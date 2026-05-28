use std::collections::HashMap;

use qjs_ast::{AssignmentOp, AssignmentTarget, BinaryOp, Expr, UpdateOp};

use crate::{RuntimeError, Value, is_truthy, operations, to_number};

use super::eval_expr;
use super::member::{assign_member, eval_member};

pub(crate) fn assign_target(
    target: &AssignmentTarget,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    match target {
        AssignmentTarget::Identifier { name, .. } => {
            if !env.contains_key(name) {
                return Err(RuntimeError {
                    message: format!("undefined identifier `{name}`"),
                });
            }
            env.insert(name.clone(), value);
            Ok(())
        }
        AssignmentTarget::Member {
            object, property, ..
        } => {
            let object = eval_expr(object, env)?;
            assign_member(object, property, value, env)
        }
    }
}

fn read_target(
    target: &AssignmentTarget,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match target {
        AssignmentTarget::Identifier { name, .. } => {
            env.get(name).cloned().ok_or_else(|| RuntimeError {
                message: format!("undefined identifier `{name}`"),
            })
        }
        AssignmentTarget::Member {
            object, property, ..
        } => {
            let object = eval_expr(object, env)?;
            eval_member(object, property, env)
        }
    }
}

pub(super) fn eval_assignment(
    target: &AssignmentTarget,
    op: AssignmentOp,
    right: &Expr,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let old_value = match op {
        AssignmentOp::LogicalAndAssign
        | AssignmentOp::LogicalOrAssign
        | AssignmentOp::NullishAssign => read_target(target, env)?,
        _ => Value::Undefined,
    };

    match op {
        AssignmentOp::LogicalAndAssign if !is_truthy(&old_value) => return Ok(old_value),
        AssignmentOp::LogicalOrAssign if is_truthy(&old_value) => return Ok(old_value),
        AssignmentOp::NullishAssign if !matches!(old_value, Value::Null | Value::Undefined) => {
            return Ok(old_value);
        }
        _ => {}
    }

    let right = eval_expr(right, env)?;
    let value = match op {
        AssignmentOp::Assign => right,
        AssignmentOp::AddAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Add, right, env)?
        }
        AssignmentOp::SubAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Sub, right, env)?
        }
        AssignmentOp::MulAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Mul, right, env)?
        }
        AssignmentOp::PowAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Pow, right, env)?
        }
        AssignmentOp::DivAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Div, right, env)?
        }
        AssignmentOp::RemAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Rem, right, env)?
        }
        AssignmentOp::ShlAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Shl, right, env)?
        }
        AssignmentOp::ShrAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Shr, right, env)?
        }
        AssignmentOp::UShrAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::UShr, right, env)?
        }
        AssignmentOp::BitwiseAndAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::BitwiseAnd, right, env)?
        }
        AssignmentOp::BitwiseXorAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::BitwiseXor, right, env)?
        }
        AssignmentOp::BitwiseOrAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::BitwiseOr, right, env)?
        }
        AssignmentOp::LogicalAndAssign
        | AssignmentOp::LogicalOrAssign
        | AssignmentOp::NullishAssign => right,
    };
    assign_target(target, value.clone(), env)?;
    Ok(value)
}

pub(super) fn eval_update(
    target: &AssignmentTarget,
    op: UpdateOp,
    prefix: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let old_number = to_number(read_target(target, env)?)?;
    let new = match op {
        UpdateOp::Increment => Value::Number(old_number + 1.0),
        UpdateOp::Decrement => Value::Number(old_number - 1.0),
    };
    assign_target(target, new.clone(), env)?;
    if prefix {
        Ok(new)
    } else {
        Ok(Value::Number(old_number))
    }
}
