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
    let value = match try_to_property_key_without_coercion(value) {
        Ok(key) => return Ok(key),
        Err(value) => value,
    };
    let primitive = match value {
        Value::Function(_)
        | Value::Array(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Object(_)
        | Value::Proxy(_) => to_primitive_with_hint(value, PreferredType::String, env)?,
        value => value,
    };

    try_to_property_key_without_coercion(primitive).map_err(|_| RuntimeError {
        thrown: None,
        message: "unsupported property key".to_owned(),
    })
}

/// Converts values whose property-key conversion cannot execute JavaScript.
/// Object-like values are returned unchanged for the caller's coercion path.
pub(crate) fn try_to_property_key_without_coercion(value: Value) -> Result<PropertyKey, Value> {
    match value {
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
        value @ (Value::Function(_)
        | Value::Array(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Object(_)
        | Value::Proxy(_)) => Err(value),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{PropertyKey, try_to_property_key_without_coercion};
    use crate::{ObjectRef, Value};

    #[test]
    fn primitive_property_keys_need_no_observable_coercion() {
        for (value, expected) in [
            (Value::String("name".to_owned().into()), "name"),
            (Value::Number(12.5), "12.5"),
            (Value::bigint(42.into()), "42"),
            (Value::Boolean(true), "true"),
            (Value::Boolean(false), "false"),
            (Value::Null, "null"),
            (Value::Undefined, "undefined"),
        ] {
            let Ok(PropertyKey::String(actual)) = try_to_property_key_without_coercion(value)
            else {
                panic!("expected a string property key");
            };
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn ordinary_objects_still_require_observable_coercion() {
        let object = ObjectRef::new(HashMap::new());
        let Err(Value::Object(returned)) =
            try_to_property_key_without_coercion(Value::Object(object.clone()))
        else {
            panic!("expected the object coercion path");
        };
        assert!(object.ptr_eq(&returned));
    }
}
