use crate::CallEnv;
use crate::{
    ObjectRef, PreferredType, RuntimeError, Value, number, symbol, to_primitive_with_hint,
};

#[derive(Clone)]
pub(crate) enum PropertyKey {
    String(String),
    Symbol(ObjectRef),
}

impl PropertyKey {
    pub(crate) fn into_value(self) -> Value {
        match self {
            Self::String(key) => Value::String(key.into()),
            Self::Symbol(symbol) => Value::Object(symbol),
        }
    }
}

pub(crate) fn to_property_key_value(
    value: Value,
    env: &mut CallEnv,
) -> Result<PropertyKey, RuntimeError> {
    let primitive = match value {
        Value::Object(object) if symbol::is_symbol_primitive(&object) => {
            return Ok(PropertyKey::Symbol(object));
        }
        Value::Function(_)
        | Value::Array(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Object(_)
        | Value::Proxy(_) => to_primitive_with_hint(value, PreferredType::String, env)?,
        value => value,
    };

    match primitive {
        Value::String(value) => Ok(PropertyKey::String(value.to_string())),
        Value::Number(number) => Ok(PropertyKey::String(number::number_to_js_string(number))),
        Value::BigInt(value) => Ok(PropertyKey::String(value.to_string())),
        Value::Boolean(true) => Ok(PropertyKey::String("true".to_owned())),
        Value::Boolean(false) => Ok(PropertyKey::String("false".to_owned())),
        Value::Null => Ok(PropertyKey::String("null".to_owned())),
        Value::Undefined => Ok(PropertyKey::String("undefined".to_owned())),
        Value::Object(object) if symbol::is_symbol_primitive(&object) => {
            Ok(PropertyKey::Symbol(object))
        }
        Value::Function(_)
        | Value::Array(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Object(_)
        | Value::Proxy(_) => Err(RuntimeError {
            thrown: None,
            message: "unsupported property key".to_owned(),
        }),
    }
}
