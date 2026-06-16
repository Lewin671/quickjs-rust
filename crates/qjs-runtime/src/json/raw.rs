use std::collections::HashMap;

use crate::{CallEnv, ObjectRef, RuntimeError, Value, to_js_string_with_env};

use super::parser::parse_json_text;

const RAW_JSON_PROPERTY: &str = "rawJSON";

pub(crate) fn native_json_raw_json(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let text = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    reject_empty_or_padded_raw_json(&text)?;
    match parse_json_text(&text, env)? {
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

fn reject_empty_or_padded_raw_json(text: &str) -> Result<(), RuntimeError> {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return Err(raw_json_syntax_error());
    };
    if is_forbidden_edge_char(first) || text.chars().last().is_some_and(is_forbidden_edge_char) {
        return Err(raw_json_syntax_error());
    }
    Ok(())
}

fn is_forbidden_edge_char(ch: char) -> bool {
    matches!(ch, '\t' | '\n' | '\r' | ' ')
}

fn raw_json_syntax_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "SyntaxError: invalid JSON.rawJSON text".to_owned(),
    }
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
