use std::collections::HashMap;

use crate::{RuntimeError, Value};

use super::super::indexing::this_string_value;

pub(crate) fn native_string_prototype_trim(
    this_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(trim_js(&this_string_value(this_value, env)?)))
}

pub(crate) fn native_string_prototype_trim_end(
    this_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(trim_js_end(&this_string_value(
        this_value, env,
    )?)))
}

pub(crate) fn native_string_prototype_trim_start(
    this_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(trim_js_start(&this_string_value(
        this_value, env,
    )?)))
}

pub(crate) fn native_string_prototype_to_string(
    this_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(this_string_value(this_value, env)?))
}

fn trim_js(value: &str) -> String {
    trim_js_end(&trim_js_start(value))
}

fn trim_js_start(value: &str) -> String {
    value
        .trim_start_matches(is_ecmascript_trim_code_point)
        .to_owned()
}

fn trim_js_end(value: &str) -> String {
    value
        .trim_end_matches(is_ecmascript_trim_code_point)
        .to_owned()
}

fn is_ecmascript_trim_code_point(ch: char) -> bool {
    matches!(
        ch,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}
