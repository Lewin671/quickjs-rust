use std::collections::HashMap;

use crate::{RuntimeError, Value};

use super::super::indexing::this_string_value;

pub(crate) fn native_string_prototype_to_lower_case(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.to_lowercase(),
    ))
}

pub(crate) fn native_string_prototype_to_upper_case(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.to_uppercase(),
    ))
}
