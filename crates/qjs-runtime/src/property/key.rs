use crate::{RuntimeError, Value};

pub(crate) fn to_property_key(value: Value) -> Result<String, RuntimeError> {
    match value {
        Value::String(value) => Ok(value),
        Value::Number(number) if number.fract() == 0.0 => Ok(format!("{number:.0}")),
        Value::Number(number) => Ok(number.to_string()),
        Value::Boolean(true) => Ok("true".to_owned()),
        Value::Boolean(false) => Ok("false".to_owned()),
        Value::Null => Ok("null".to_owned()),
        Value::Undefined => Ok("undefined".to_owned()),
        Value::Function(_) | Value::Array(_) | Value::Object(_) => Err(RuntimeError {
            message: "unsupported property key".to_owned(),
        }),
    }
}
