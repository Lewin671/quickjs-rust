use std::collections::HashMap;

use crate::{ObjectRef, RuntimeError, Value, to_js_string};

use super::parser::parse_json_text;

const RAW_JSON_PROPERTY: &str = "rawJSON";

pub(crate) fn native_json_raw_json(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let text = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    match parse_json_text(&text, &crate::CallEnv::detached())? {
        Value::Array(_) | Value::Object(_) => {
            return Err(RuntimeError {
                thrown: None,
                message: "SyntaxError: JSON.rawJSON cannot wrap arrays or objects".to_owned(),
            });
        }
        _ => {}
    }

    let object = ObjectRef::with_prototype(HashMap::new(), None);
    object.set(RAW_JSON_PROPERTY.to_owned(), Value::String(text));
    object.mark_raw_json();
    object.freeze();
    Ok(Value::Object(object))
}

pub(crate) fn native_json_is_raw_json(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Object(object)) if object.is_raw_json()
    )))
}

pub(crate) fn raw_json_value(object: &ObjectRef) -> Option<String> {
    if !object.is_raw_json() {
        return None;
    }
    match object.own_property(RAW_JSON_PROPERTY) {
        Some(property) => match property.value {
            Value::String(value) => Some(value),
            _ => None,
        },
        None => None,
    }
}
