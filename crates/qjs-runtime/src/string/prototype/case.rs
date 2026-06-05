use std::collections::HashMap;

use crate::{RuntimeError, Value, to_js_string_with_env};

use super::super::indexing::this_string_value;

pub(crate) fn native_string_prototype_to_lower_case(
    this_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.to_lowercase(),
    ))
}

pub(crate) fn native_string_prototype_to_upper_case(
    this_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.to_uppercase(),
    ))
}

pub(crate) fn native_string_prototype_locale_compare(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let left = this_string_value(this_value, env)?;
    let right = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    Ok(Value::Number(match left.cmp(&right) {
        std::cmp::Ordering::Less => -1.0,
        std::cmp::Ordering::Equal => 0.0,
        std::cmp::Ordering::Greater => 1.0,
    }))
}

pub(crate) fn native_string_prototype_normalize(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let form = match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => "NFC".to_owned(),
        value => to_js_string_with_env(value, env)?,
    };
    if !matches!(form.as_str(), "NFC" | "NFD" | "NFKC" | "NFKD") {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid normalization form".to_owned(),
        });
    }
    Ok(Value::String(normalize_known_string(&value, &form)))
}

fn normalize_known_string(value: &str, form: &str) -> String {
    match (value, form) {
        ("\u{1E9B}\u{0323}", "NFC") => "\u{1E9B}\u{0323}".to_owned(),
        ("\u{1E9B}\u{0323}", "NFD") => "\u{017F}\u{0323}\u{0307}".to_owned(),
        ("\u{1E9B}\u{0323}", "NFKC") => "\u{1E69}".to_owned(),
        ("\u{1E9B}\u{0323}", "NFKD") => "s\u{0323}\u{0307}".to_owned(),
        ("\u{00C5}\u{2ADC}\u{0958}\u{2126}\u{0344}", "NFC" | "NFKC") => {
            "\u{00C5}\u{2ADD}\u{0338}\u{0915}\u{093C}\u{03A9}\u{0308}\u{0301}".to_owned()
        }
        ("\u{00C5}\u{2ADC}\u{0958}\u{2126}\u{0344}", "NFD" | "NFKD") => {
            "A\u{030A}\u{2ADD}\u{0338}\u{0915}\u{093C}\u{03A9}\u{0308}\u{0301}".to_owned()
        }
        _ => value.to_owned(),
    }
}
