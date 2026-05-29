use crate::{ArrayRef, ObjectRef, RuntimeError, Value, number};

pub(crate) fn native_json_stringify(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    match stringify_value(
        &argument_values.first().cloned().unwrap_or(Value::Undefined),
        false,
    )? {
        Some(json) => Ok(Value::String(json)),
        None => Ok(Value::Undefined),
    }
}

fn stringify_value(value: &Value, in_array: bool) -> Result<Option<String>, RuntimeError> {
    match value {
        Value::String(value) => Ok(Some(quote_json_string(value))),
        Value::Number(number) if number.is_finite() => {
            Ok(Some(number::number_to_js_string(*number)))
        }
        Value::Number(_) | Value::Null => Ok(Some("null".to_owned())),
        Value::Boolean(true) => Ok(Some("true".to_owned())),
        Value::Boolean(false) => Ok(Some("false".to_owned())),
        Value::Array(array) => stringify_array(array).map(Some),
        Value::Object(object) => stringify_object(object).map(Some),
        Value::Undefined | Value::Function(_) if in_array => Ok(Some("null".to_owned())),
        Value::Undefined | Value::Function(_) => Ok(None),
    }
}

fn stringify_array(array: &ArrayRef) -> Result<String, RuntimeError> {
    let mut parts = Vec::new();
    for element in array.to_vec() {
        parts.push(stringify_value(&element, true)?.unwrap_or_else(|| "null".to_owned()));
    }
    Ok(format!("[{}]", parts.join(",")))
}

fn stringify_object(object: &ObjectRef) -> Result<String, RuntimeError> {
    let mut parts = Vec::new();
    for key in object.own_property_keys() {
        let Some(value) = object.own_property(&key).map(|property| property.value) else {
            continue;
        };
        let Some(json) = stringify_value(&value, false)? else {
            continue;
        };
        parts.push(format!("{}:{json}", quote_json_string(&key)));
    }
    Ok(format!("{{{}}}", parts.join(",")))
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
