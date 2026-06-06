use crate::{ObjectRef, RuntimeError, Value, number, symbol};

#[derive(Clone)]
pub(crate) enum PropertyKey {
    String(String),
    Symbol(ObjectRef),
}

pub(crate) fn to_property_key_value(value: Value) -> Result<PropertyKey, RuntimeError> {
    match value {
        Value::String(value) => Ok(PropertyKey::String(value)),
        Value::Number(number) => Ok(PropertyKey::String(number::number_to_js_string(number))),
        Value::Boolean(true) => Ok(PropertyKey::String("true".to_owned())),
        Value::Boolean(false) => Ok(PropertyKey::String("false".to_owned())),
        Value::Null => Ok(PropertyKey::String("null".to_owned())),
        Value::Undefined => Ok(PropertyKey::String("undefined".to_owned())),
        Value::Object(object) if symbol::is_symbol_object(&object) => {
            Ok(PropertyKey::Symbol(object))
        }
        Value::Function(_) | Value::Array(_) | Value::Map(_) | Value::Set(_) | Value::Object(_) => {
            Err(RuntimeError {
                thrown: None,
                message: "unsupported property key".to_owned(),
            })
        }
    }
}
