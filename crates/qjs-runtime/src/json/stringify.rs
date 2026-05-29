use std::collections::HashMap;

use crate::{ArrayRef, ObjectRef, RuntimeError, Value, call_function, number};

pub(crate) fn native_json_stringify(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match stringify_value(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        "",
        false,
        env,
    )? {
        Some(json) => Ok(Value::String(json)),
        None => Ok(Value::Undefined),
    }
}

fn stringify_value(
    value: Value,
    key: &str,
    in_array: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Option<String>, RuntimeError> {
    let value = apply_to_json(value, key, env)?;
    match &value {
        Value::String(value) => Ok(Some(quote_json_string(value))),
        Value::Number(number) if number.is_finite() => {
            Ok(Some(number::number_to_js_string(*number)))
        }
        Value::Number(_) | Value::Null => Ok(Some("null".to_owned())),
        Value::Boolean(true) => Ok(Some("true".to_owned())),
        Value::Boolean(false) => Ok(Some("false".to_owned())),
        Value::Array(array) => stringify_array(array, env).map(Some),
        Value::Object(object) => stringify_object(object, env).map(Some),
        Value::Undefined | Value::Function(_) if in_array => Ok(Some("null".to_owned())),
        Value::Undefined | Value::Function(_) => Ok(None),
    }
}

fn stringify_array(
    array: &ArrayRef,
    env: &mut HashMap<String, Value>,
) -> Result<String, RuntimeError> {
    let mut parts = Vec::new();
    for (index, element) in array.to_vec().into_iter().enumerate() {
        parts.push(
            stringify_value(element, &index.to_string(), true, env)?
                .unwrap_or_else(|| "null".to_owned()),
        );
    }
    Ok(format!("[{}]", parts.join(",")))
}

fn stringify_object(
    object: &ObjectRef,
    env: &mut HashMap<String, Value>,
) -> Result<String, RuntimeError> {
    let mut parts = Vec::new();
    for key in object.own_property_keys() {
        let Some(value) = object.own_property(&key).map(|property| property.value) else {
            continue;
        };
        let Some(json) = stringify_value(value, &key, false, env)? else {
            continue;
        };
        parts.push(format!("{}:{json}", quote_json_string(&key)));
    }
    Ok(format!("{{{}}}", parts.join(",")))
}

fn apply_to_json(
    value: Value,
    key: &str,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Object(object) = &value else {
        return Ok(value);
    };
    let Some(to_json) = object.get("toJSON") else {
        return Ok(value);
    };
    call_function(
        to_json,
        value,
        vec![Value::String(key.to_owned())],
        env,
        false,
    )
}

fn quote_json_string(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch <= '\u{1f}' => output.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => output.push(ch),
        }
    }
    output.push('"');
    output
}
