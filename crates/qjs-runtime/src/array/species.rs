use std::collections::HashMap;

use crate::{RuntimeError, Value, property_value};

pub(super) fn validate_array_species_constructor(
    receiver: Value,
    method: &str,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    if !matches!(receiver, Value::Array(_)) {
        return Ok(());
    }

    match property_value(receiver, "constructor", env)? {
        Value::Undefined | Value::Function(_) | Value::Object(_) => Ok(()),
        _ => Err(RuntimeError {
            thrown: None,
            message: format!(
                "TypeError: Array.prototype.{method} constructor is not a constructor"
            ),
        }),
    }
}
