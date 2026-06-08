use crate::{RuntimeError, Value, symbol};

pub(super) fn ensure_reflect_object_target(
    target: &Value,
    method: &str,
) -> Result<(), RuntimeError> {
    match target {
        Value::Object(object) if symbol::is_symbol_primitive(object) => Err(RuntimeError {
            thrown: None,
            message: format!("{method} target must be an object"),
        }),
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) => {
            Ok(())
        }
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: format!("{method} target must be an object"),
        }),
    }
}
