use std::collections::HashMap;

use qjs_ast::{ForInLeft, ForInit, Stmt};

use crate::{Function, RuntimeError, Value, assign_target, eval_expr, is_truthy};

mod control;
mod declarations;

pub(crate) use declarations::{collect_function_local_names, hoist_declarations};

use control::{eval_switch, eval_try};

pub(crate) enum Completion {
    Normal(Value),
    Return(Value),
    Break,
    Continue,
    Throw(Value),
}

pub(crate) fn eval_stmt(
    stmt: &Stmt,
    env: &mut HashMap<String, Value>,
) -> Result<Completion, RuntimeError> {
    match stmt {
        Stmt::Expr(expr) => eval_expr(expr, env).map(Completion::Normal),
        Stmt::Block { body, .. } => eval_statement_list(body, env),
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            let test = eval_expr(test, env)?;
            if is_truthy(&test) {
                eval_stmt(consequent, env)
            } else if let Some(alternate) = alternate {
                eval_stmt(alternate, env)
            } else {
                Ok(Completion::Normal(Value::Undefined))
            }
        }
        Stmt::While { test, body, .. } => {
            let mut last = Value::Undefined;
            while is_truthy(&eval_expr(test, env)?) {
                match eval_stmt(body, env)? {
                    Completion::Normal(value) => last = value,
                    Completion::Return(value) => return Ok(Completion::Return(value)),
                    Completion::Break => break,
                    Completion::Continue => {}
                    Completion::Throw(value) => return Ok(Completion::Throw(value)),
                }
            }
            Ok(Completion::Normal(last))
        }
        Stmt::DoWhile { body, test, .. } => {
            let mut last = Value::Undefined;
            loop {
                match eval_stmt(body, env)? {
                    Completion::Normal(value) => last = value,
                    Completion::Return(value) => return Ok(Completion::Return(value)),
                    Completion::Break => break,
                    Completion::Continue => {}
                    Completion::Throw(value) => return Ok(Completion::Throw(value)),
                }
                if !is_truthy(&eval_expr(test, env)?) {
                    break;
                }
            }
            Ok(Completion::Normal(last))
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            if let Some(init) = init {
                eval_for_init(init, env)?;
            }
            let mut last = Value::Undefined;
            while test.as_ref().map_or(Ok(true), |test| {
                eval_expr(test, env).map(|value| is_truthy(&value))
            })? {
                match eval_stmt(body, env)? {
                    Completion::Normal(value) => last = value,
                    Completion::Return(value) => return Ok(Completion::Return(value)),
                    Completion::Break => break,
                    Completion::Continue => {}
                    Completion::Throw(value) => return Ok(Completion::Throw(value)),
                }
                if let Some(update) = update {
                    eval_expr(update, env)?;
                }
            }
            Ok(Completion::Normal(last))
        }
        Stmt::ForIn {
            left, right, body, ..
        } => {
            let keys = enumerable_keys(eval_expr(right, env)?)?;
            let mut last = Value::Undefined;
            for key in keys {
                assign_for_in_left(left, Value::String(key), env)?;
                match eval_stmt(body, env)? {
                    Completion::Normal(value) => last = value,
                    Completion::Return(value) => return Ok(Completion::Return(value)),
                    Completion::Break => break,
                    Completion::Continue => {}
                    Completion::Throw(value) => return Ok(Completion::Throw(value)),
                }
            }
            Ok(Completion::Normal(last))
        }
        Stmt::Switch {
            discriminant,
            cases,
            ..
        } => eval_switch(discriminant, cases, env),
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => eval_try(block, handler.as_ref(), finalizer.as_deref(), env),
        Stmt::FunctionDecl {
            name, params, body, ..
        } => {
            env.insert(
                name.clone(),
                Value::Function(Function::new_user(
                    Some(name.clone()),
                    params.clone(),
                    body.clone(),
                    env.clone(),
                )),
            );
            Ok(Completion::Normal(Value::Undefined))
        }
        Stmt::Return { argument, .. } => {
            let value = if let Some(argument) = argument {
                eval_expr(argument, env)?
            } else {
                Value::Undefined
            };
            Ok(Completion::Return(value))
        }
        Stmt::Throw { argument, .. } => {
            let value = if let Some(argument) = argument {
                eval_expr(argument, env)?
            } else {
                Value::Undefined
            };
            Ok(Completion::Throw(value))
        }
        Stmt::Debugger { .. } => Ok(Completion::Normal(Value::Undefined)),
        Stmt::Break { .. } => Ok(Completion::Break),
        Stmt::Continue { .. } => Ok(Completion::Continue),
        Stmt::VarDecl { declarations, .. } => {
            for declaration in declarations {
                let value = if let Some(init) = &declaration.init {
                    eval_expr(init, env)?
                } else {
                    Value::Undefined
                };
                env.insert(declaration.name.clone(), value);
            }
            Ok(Completion::Normal(Value::Undefined))
        }
        Stmt::Empty => Ok(Completion::Normal(Value::Undefined)),
    }
}

pub(crate) fn eval_statement_list(
    body: &[Stmt],
    env: &mut HashMap<String, Value>,
) -> Result<Completion, RuntimeError> {
    hoist_declarations(body, env);
    let mut last = Value::Undefined;
    for stmt in body {
        match eval_stmt(stmt, env)? {
            Completion::Normal(value) => last = value,
            Completion::Return(value) => return Ok(Completion::Return(value)),
            Completion::Break => return Ok(Completion::Break),
            Completion::Continue => return Ok(Completion::Continue),
            Completion::Throw(value) => return Ok(Completion::Throw(value)),
        }
    }
    Ok(Completion::Normal(last))
}

fn eval_for_init(init: &ForInit, env: &mut HashMap<String, Value>) -> Result<(), RuntimeError> {
    match init {
        ForInit::VarDecl { declarations, .. } => {
            for declaration in declarations {
                let value = if let Some(init) = &declaration.init {
                    eval_expr(init, env)?
                } else {
                    Value::Undefined
                };
                env.insert(declaration.name.clone(), value);
            }
            Ok(())
        }
        ForInit::Expr(expr) => eval_expr(expr, env).map(|_| ()),
    }
}

fn assign_for_in_left(
    left: &ForInLeft,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    match left {
        ForInLeft::VarDecl { name, .. } => {
            env.insert(name.clone(), value);
            Ok(())
        }
        ForInLeft::Target(target) => assign_target(target, value, env),
    }
}

fn enumerable_keys(value: Value) -> Result<Vec<String>, RuntimeError> {
    match value {
        Value::Object(object) => Ok(object.own_property_keys()),
        Value::Array(elements) => Ok((0..elements.len()).map(|index| index.to_string()).collect()),
        Value::Null | Value::Undefined => Ok(Vec::new()),
        _ => Err(RuntimeError {
            message: "for-in target is not enumerable".to_owned(),
        }),
    }
}
