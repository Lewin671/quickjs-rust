use std::collections::HashMap;

use qjs_ast::Stmt;
use qjs_parser::parse_script;

use crate::{
    Function, GLOBAL_THIS_BINDING, RuntimeError, Value, array::array_like_values_with_env,
    object::boxed_primitive, property_value, to_js_string_with_env, to_length_with_env,
};

pub(crate) fn native_function(
    _function: &Function,
    _this_value: Value,
    argument_values: &[Value],
    _is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let (params, body) = function_source_parts(argument_values, env)?;
    let source = format!("function anonymous({params}) {{\n{body}\n}}");
    let script = parse_script(&source).map_err(|error| RuntimeError {
        thrown: None,
        message: format!("invalid Function constructor source: {}", error.message),
    })?;

    let Some(Stmt::FunctionDecl {
        name, params, body, ..
    }) = script.body.into_iter().next()
    else {
        return Err(RuntimeError {
            thrown: None,
            message: "Function constructor did not produce a function declaration".to_owned(),
        });
    };

    Ok(Value::Function(Function::new_user(
        Some(name),
        params,
        body,
        env.clone(),
    )?))
}

fn function_source_parts(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<(String, String), RuntimeError> {
    let Some((body, params)) = argument_values.split_last() else {
        return Ok((String::new(), String::new()));
    };

    let params = params
        .iter()
        .cloned()
        .map(|value| to_js_string_with_env(value, env))
        .collect::<Result<Vec<_>, _>>()?
        .join(",");
    let body = to_js_string_with_env(body.clone(), env)?;
    Ok((params, body))
}

pub(crate) fn native_function_prototype_apply(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Function(_) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Function.prototype.apply target is not callable".to_owned(),
        });
    };

    let call_this = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let apply_arguments = apply_argument_list(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;

    crate::call_function(this_value, call_this, apply_arguments, env, false)
}

fn apply_argument_list(
    arg_array: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Vec<Value>, RuntimeError> {
    match arg_array {
        Value::Null | Value::Undefined => Ok(Vec::new()),
        Value::Array(array) => {
            let receiver = Value::Array(array.clone());
            (0..array.len())
                .map(|index| property_value(receiver.clone(), &index.to_string(), env))
                .collect()
        }
        Value::Object(object) if object.to_string_tag().as_deref() == Some("Symbol") => {
            Err(apply_argument_list_type_error())
        }
        Value::Object(_) => {
            array_like_values_with_env(arg_array, "Function.prototype.apply argument list", env)
        }
        Value::Function(_) => {
            let length =
                to_length_with_env(property_value(arg_array.clone(), "length", env)?, env)?;
            (0..length)
                .map(|index| property_value(arg_array.clone(), &index.to_string(), env))
                .collect()
        }
        Value::String(_) | Value::Number(_) | Value::Boolean(_) | Value::Map(_) | Value::Set(_) => {
            Err(apply_argument_list_type_error())
        }
    }
}

fn apply_argument_list_type_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Function.prototype.apply argument list must be an object".to_owned(),
    }
}

pub(crate) fn native_function_prototype_call(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Function(_) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Function.prototype.call target is not callable".to_owned(),
        });
    };

    let call_this = argument_values.first().cloned().unwrap_or(Value::Undefined);
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
            thrown: None,
            message: "Function.prototype.bind target is not callable".to_owned(),
        });
    };

    let bound_this = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let bound_arguments = argument_values.iter().skip(1).cloned().collect::<Vec<_>>();
    let length = target.params.len().saturating_sub(bound_arguments.len());
    let bound = Function::new_bound(this_value, bound_this, bound_arguments, length);
    Ok(Value::Function(bound))
}

pub(crate) fn native_function_prototype_has_instance(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if !matches!(this_value, Value::Function(_)) {
        return Ok(Value::Boolean(false));
    }
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    crate::operations::ordinary_has_instance(value, this_value, env).map(Value::Boolean)
}

pub(crate) fn native_function_prototype_to_string(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let Value::Function(function) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Function.prototype.toString requires a callable receiver".to_owned(),
        });
    };
    let name = function.name.clone().unwrap_or_default();
    Ok(Value::String(format!(
        "function {name}() {{ [native code] }}"
    )))
}

pub(crate) fn function_call_this(
    this_arg: Option<Value>,
    env: &HashMap<String, Value>,
    is_strict: bool,
) -> Value {
    let this_value = this_arg.unwrap_or(Value::Undefined);
    match this_value {
        Value::Null | Value::Undefined if !is_strict => env
            .get(GLOBAL_THIS_BINDING)
            .cloned()
            .unwrap_or(Value::Undefined),
        Value::String(_) | Value::Number(_) | Value::Boolean(_) if !is_strict => {
            boxed_primitive(this_value, env).expect("primitive value should box")
        }
        value => value,
    }
}
