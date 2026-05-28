use std::collections::HashMap;

use crate::{RuntimeError, Value};

use super::super::indexing::this_string_value;

pub(crate) fn native_string_prototype_trim(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.trim().to_owned(),
    ))
}

pub(crate) fn native_string_prototype_trim_end(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.trim_end().to_owned(),
    ))
}

pub(crate) fn native_string_prototype_trim_start(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.trim_start().to_owned(),
    ))
}

pub(crate) fn native_string_prototype_to_string(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(this_string_value(this_value, env)?))
}
