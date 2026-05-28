use std::collections::HashMap;

use qjs_ast::{CatchClause, Expr, Stmt, SwitchCase};

use crate::{RuntimeError, Value, eval_expr};

use super::{Completion, eval_statement_list, eval_stmt};

pub(super) fn eval_switch(
    discriminant: &Expr,
    cases: &[SwitchCase],
    env: &mut HashMap<String, Value>,
) -> Result<Completion, RuntimeError> {
    let discriminant = eval_expr(discriminant, env)?;
    let mut default_index = None;
    let mut selected_index = None;

    for (index, case) in cases.iter().enumerate() {
        if let Some(test) = &case.test {
            if eval_expr(test, env)? == discriminant {
                selected_index = Some(index);
                break;
            }
        } else {
            default_index = Some(index);
        }
    }

    let Some(start_index) = selected_index.or(default_index) else {
        return Ok(Completion::Normal(Value::Undefined));
    };

    let mut last = Value::Undefined;
    for case in &cases[start_index..] {
        for stmt in &case.consequent {
            match eval_stmt(stmt, env)? {
                Completion::Normal(value) => last = value,
                Completion::Break => return Ok(Completion::Normal(last)),
                Completion::Return(value) => return Ok(Completion::Return(value)),
                Completion::Continue => return Ok(Completion::Continue),
                Completion::Throw(value) => return Ok(Completion::Throw(value)),
            }
        }
    }
    Ok(Completion::Normal(last))
}

pub(super) fn eval_try(
    block: &[Stmt],
    handler: Option<&CatchClause>,
    finalizer: Option<&[Stmt]>,
    env: &mut HashMap<String, Value>,
) -> Result<Completion, RuntimeError> {
    let mut completion = match eval_statement_list(block, env)? {
        Completion::Throw(value) => {
            if let Some(handler) = handler {
                eval_catch(handler, value, env)?
            } else {
                Completion::Throw(value)
            }
        }
        other => other,
    };

    if let Some(finalizer) = finalizer {
        let final_completion = eval_statement_list(finalizer, env)?;
        completion = match final_completion {
            Completion::Normal(_) => completion,
            abrupt => abrupt,
        };
    }

    Ok(completion)
}

fn eval_catch(
    handler: &CatchClause,
    thrown: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Completion, RuntimeError> {
    let previous = if let Some(param) = &handler.param {
        env.insert(param.clone(), thrown)
    } else {
        None
    };
    let completion = eval_statement_list(&handler.body, env);
    if let Some(param) = &handler.param {
        if let Some(value) = previous {
            env.insert(param.clone(), value);
        } else {
            env.remove(param);
        }
    }
    completion
}
