use std::collections::HashMap;

use qjs_ast::Stmt;
use qjs_parser::parse_script;

use crate::{Function, GLOBAL_THIS_BINDING, RuntimeError, Value, to_js_string};

pub(crate) fn native_function(
    _function: &Function,
    _this_value: Value,
    argument_values: &[Value],
    _is_construct: bool,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let (params, body) = function_source_parts(argument_values)?;
    let source = format!("function anonymous({params}) {{\n{body}\n}}");
    let script = parse_script(&source).map_err(|error| RuntimeError {
        message: format!("invalid Function constructor source: {}", error.message),
    })?;

    let Some(Stmt::FunctionDecl {
        name, params, body, ..
    }) = script.body.into_iter().next()
    else {
        return Err(RuntimeError {
            message: "Function constructor did not produce a function declaration".to_owned(),
        });
    };

    Ok(Value::Function(Function::new_user(
        Some(name),
        params,
        body,
        env.clone(),
    )))
}

fn function_source_parts(argument_values: &[Value]) -> Result<(String, String), RuntimeError> {
    let Some((body, params)) = argument_values.split_last() else {
        return Ok((String::new(), String::new()));
    };

    let params = params
        .iter()
        .cloned()
        .map(to_js_string)
        .collect::<Result<Vec<_>, _>>()?
        .join(",");
    let body = to_js_string(body.clone())?;
    Ok((params, body))
}

pub(crate) fn native_function_prototype_apply(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Function(_) = this_value else {
        return Err(RuntimeError {
            message: "Function.prototype.apply target is not callable".to_owned(),
        });
    };

    let call_this = function_call_this(argument_values.first().cloned(), env);
    let apply_arguments = match argument_values.get(1).cloned().unwrap_or(Value::Undefined) {
        Value::Null | Value::Undefined => Vec::new(),
        Value::Array(elements) => elements.to_vec(),
        value => {
            return Err(RuntimeError {
                message: format!(
                    "Function.prototype.apply argument list is not array-like: {value:?}"
                ),
            });
        }
    };

    crate::call_function(this_value, call_this, apply_arguments, env, false)
}

pub(crate) fn native_function_prototype_call(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Function(_) = this_value else {
        return Err(RuntimeError {
            message: "Function.prototype.call target is not callable".to_owned(),
        });
    };

    let call_this = function_call_this(argument_values.first().cloned(), env);
    crate::call_function(
        this_value,
        call_this,
        argument_values.iter().skip(1).cloned().collect(),
        env,
        false,
    )
}

pub(crate) fn native_function_prototype_bind(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Function(target) = this_value.clone() else {
        return Err(RuntimeError {
            message: "Function.prototype.bind target is not callable".to_owned(),
        });
    };

    let bound_this = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let bound_arguments = argument_values.iter().skip(1).cloned().collect::<Vec<_>>();
    let length = target.params.len().saturating_sub(bound_arguments.len());
    let bound = Function::new_bound(this_value, bound_this, bound_arguments, length);
    Ok(Value::Function(bound))
}

pub(crate) fn function_call_this(this_arg: Option<Value>, env: &HashMap<String, Value>) -> Value {
    match this_arg.unwrap_or(Value::Undefined) {
        Value::Null | Value::Undefined => env
            .get(GLOBAL_THIS_BINDING)
            .cloned()
            .unwrap_or(Value::Undefined),
        value => value,
    }
}
