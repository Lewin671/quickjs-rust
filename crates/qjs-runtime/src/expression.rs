use std::collections::HashMap;

use qjs_ast::{
    AssignmentOp, AssignmentTarget, BinaryOp, Expr, Literal, MemberProperty, ObjectPropertyKey,
    UnaryOp, UpdateOp,
};

use crate::{
    ArrayRef, Function, GLOBAL_THIS_BINDING, ObjectRef, Property, RuntimeError, Value, boolean,
    call_function, constructor_prototype, inherited_array_prototype_property,
    inherited_object_prototype_property, inherited_string_prototype_property, is_truthy, number,
    object_prototype, operations, string, to_array_index, to_length, to_number, to_property_key,
};

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

fn eval_assignment(
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

fn eval_update(
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

fn eval_literal(literal: &Literal) -> Result<Value, RuntimeError> {
    match literal {
        Literal::Number { raw, .. } => {
            raw.parse::<f64>()
                .map(Value::Number)
                .map_err(|_| RuntimeError {
                    message: format!("invalid number literal `{raw}`"),
                })
        }
        Literal::String { value, .. } => Ok(Value::String(value.clone())),
        Literal::Boolean { value, .. } => Ok(Value::Boolean(*value)),
        Literal::Null { .. } => Ok(Value::Null),
    }
}

fn eval_member(
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
                .or_else(|| inherited_object_prototype_property(env, &key))
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

fn assign_member(
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

fn eval_delete(expr: &Expr, env: &mut HashMap<String, Value>) -> Result<Value, RuntimeError> {
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
