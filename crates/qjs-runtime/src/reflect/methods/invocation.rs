use std::collections::HashMap;

use crate::{
    RuntimeError, Value, array::array_like_values_with_env, construct_function, ensure_constructor,
};

pub(crate) fn native_reflect_apply(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(target, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "Reflect.apply target is not callable".to_owned(),
        });
    }

    let this_value = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let arguments_list = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_argument_list(&arguments_list, "Reflect.apply")?;
    let arguments = array_like_values_with_env(arguments_list, "Reflect.apply argument list", env)?;

    crate::call_function(target, this_value, arguments, env, false)
}

pub(crate) fn native_reflect_construct(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_constructor(&target, "target")?;

    let arguments_list = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_argument_list(&arguments_list, "Reflect.construct")?;
    let arguments =
        array_like_values_with_env(arguments_list, "Reflect.construct argument list", env)?;

    let new_target = argument_values
        .get(2)
        .cloned()
        .unwrap_or_else(|| target.clone());
    ensure_reflect_constructor(&new_target, "newTarget")?;

    construct_function(target, new_target, arguments, env)
}

fn ensure_reflect_constructor(value: &Value, name: &str) -> Result<(), RuntimeError> {
    ensure_constructor(value).map_err(|_| RuntimeError {
        thrown: None,
        message: format!("Reflect.construct {name} is not a constructor"),
    })
}

fn ensure_reflect_object_argument_list(value: &Value, name: &str) -> Result<(), RuntimeError> {
    match value {
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) => {
            Ok(())
        }
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: format!("{name} argument list must be an object"),
        }),
    }
}
