use crate::CallEnv;
use crate::{
    PropertyKey, RuntimeError, Value, array::array_like_values_from_receiver, construct_function,
    ensure_constructor, property_value_key, symbol, to_length_with_env,
};

pub(crate) fn native_reflect_apply(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let callable = match &target {
        Value::Function(_) => true,
        Value::Proxy(proxy) => crate::proxy::proxy_is_callable(proxy),
        _ => false,
    };
    if !callable {
        return Err(RuntimeError {
            thrown: None,
            message: "Reflect.apply target is not callable".to_owned(),
        });
    }

    let this_value = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let arguments_list = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    let arguments = reflect_argument_list(arguments_list, "Reflect.apply", env)?;

    crate::call_function(target, this_value, arguments, env, false)
}

pub(crate) fn native_reflect_construct(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_constructor(&target, "target")?;

    let arguments_list = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let arguments = reflect_argument_list(arguments_list, "Reflect.construct", env)?;

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
        Value::Object(object) if symbol::is_symbol_primitive(object) => Err(RuntimeError {
            thrown: None,
            message: format!("{name} argument list must be an object"),
        }),
        Value::Object(_)
        | Value::Array(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Proxy(_) => Ok(()),
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: format!("{name} argument list must be an object"),
        }),
    }
}

fn reflect_argument_list(
    value: Value,
    name: &str,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    ensure_reflect_object_argument_list(&value, name)?;
    match value {
        Value::Array(array) => Ok(array.to_vec()),
        value @ (Value::Object(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Proxy(_)) => {
            let length = to_length_with_env(
                property_value_key(
                    value.clone(),
                    &PropertyKey::String("length".to_owned()),
                    env,
                )?,
                env,
            )?;
            array_like_values_from_receiver(value, length, env)
        }
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => unreachable!("argument list was validated before collection"),
    }
}
