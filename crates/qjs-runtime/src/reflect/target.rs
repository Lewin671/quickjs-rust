use crate::{RuntimeError, Value};

pub(super) fn ensure_reflect_object_target(
    target: &Value,
    method: &str,
) -> Result<(), RuntimeError> {
    match target {
        Value::Object(_) | Value::Array(_) | Value::Function(_) => Ok(()),
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Err(RuntimeError {
            message: format!("{method} target must be an object"),
        }),
    }
}
