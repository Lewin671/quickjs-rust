use std::collections::HashMap;

use crate::{
    PropertyKey, RuntimeError, Value, call_function, property_value, reflect, symbol,
    to_js_string_with_env,
};

pub(crate) fn native_regexp_prototype_search(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if !is_object_value(&this_value) {
        return Err(RuntimeError {
            thrown: None,
            message: "RegExp.prototype[Symbol.search] requires an object receiver".to_owned(),
        });
    }
    let input = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let previous_last_index = property_value(this_value.clone(), "lastIndex", env)?;
    if !previous_last_index.same_value(&Value::Number(0.0)) {
        set_last_index(this_value.clone(), Value::Number(0.0), env)?;
    }
    let exec = property_value(this_value.clone(), "exec", env)?;
    let result = call_function(
        exec,
        this_value.clone(),
        vec![Value::String(input)],
        env,
        false,
    )?;
    let current_last_index = property_value(this_value.clone(), "lastIndex", env)?;
    if !current_last_index.same_value(&previous_last_index) {
        set_last_index(this_value.clone(), previous_last_index, env)?;
    }
    match result {
        Value::Array(array) => property_value(Value::Array(array), "index", env),
        Value::Object(object) if symbol::is_symbol_primitive(&object) => Err(exec_result_error()),
        Value::Object(object) => property_value(Value::Object(object), "index", env),
        Value::Function(function) => property_value(Value::Function(function), "index", env),
        Value::Map(map) => property_value(Value::Map(map), "index", env),
        Value::Set(set) => property_value(Value::Set(set), "index", env),
        Value::Null => Ok(Value::Number(-1.0)),
        _ => Err(exec_result_error()),
    }
}

fn exec_result_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: RegExp exec must return an object or null".to_owned(),
    }
}

fn set_last_index(
    receiver: Value,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    if reflect::ordinary_set(
        receiver.clone(),
        &PropertyKey::String("lastIndex".to_owned()),
        value,
        receiver,
        env,
    )? {
        Ok(())
    } else {
        Err(RuntimeError {
            thrown: None,
            message: "TypeError: RegExp.prototype[Symbol.search] cannot set lastIndex".to_owned(),
        })
    }
}

fn is_object_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(object) if !symbol::is_symbol_primitive(object)
    ) || matches!(
        value,
        Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    )
}
