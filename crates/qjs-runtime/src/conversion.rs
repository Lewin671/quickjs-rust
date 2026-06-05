use std::collections::HashMap;

use crate::{RuntimeError, Value, array::array_join, call_function, number, property_value};

pub(crate) fn to_js_string(value: Value) -> Result<String, RuntimeError> {
    let mut env = HashMap::new();
    to_js_string_with_env(value, &mut env)
}

pub(crate) fn to_js_string_with_env(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<String, RuntimeError> {
    match value {
        Value::Number(number) => Ok(number::number_to_js_string(number)),
        Value::String(value) => Ok(value),
        Value::Boolean(true) => Ok("true".to_owned()),
        Value::Boolean(false) => Ok("false".to_owned()),
        Value::Null => Ok("null".to_owned()),
        Value::Undefined => Ok("undefined".to_owned()),
        Value::Object(object) => match crate::string_object_value(&object) {
            Some(value) => Ok(value),
            None => object_to_string(Value::Object(object), env),
        },
        Value::Array(array) => array_join(Value::Array(array), ",", env),
        Value::Function(_) => object_to_string(value, env),
    }
}

pub(crate) fn error_value(value: Value) -> String {
    match value {
        Value::Number(number) => number::number_to_js_string(number),
        Value::String(value) => value,
        Value::Boolean(true) => "true".to_owned(),
        Value::Boolean(false) => "false".to_owned(),
        Value::Null => "null".to_owned(),
        Value::Undefined => "undefined".to_owned(),
        Value::Function(_) => "function".to_owned(),
        Value::Array(_) => "array".to_owned(),
        Value::Object(object) => {
            crate::error::error_object_to_string(&object).unwrap_or_else(|| "object".to_owned())
        }
    }
}

pub(crate) fn to_number(value: Value) -> Result<f64, RuntimeError> {
    let mut env = HashMap::new();
    to_number_with_env(value, &mut env)
}

pub(crate) fn to_number_with_env(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(number) => Ok(number),
        Value::Boolean(true) => Ok(1.0),
        Value::Boolean(false) | Value::Null => Ok(0.0),
        Value::String(value) => string_to_number(&value),
        Value::Undefined => Ok(f64::NAN),
        Value::Object(object) => match crate::string_object_value(&object) {
            Some(value) => string_to_number(&value),
            None => object_to_number(Value::Object(object), env),
        },
        Value::Function(_) => object_to_number(value, env),
        Value::Array(array) => {
            string_to_number(&array_join(Value::Array(array), ",", &mut HashMap::new())?)
        }
    }
}

pub(crate) fn to_primitive_with_env(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match value {
        Value::Object(_) | Value::Function(_) | Value::Array(_) => object_to_primitive(value, env),
        value => Ok(value),
    }
}

fn string_to_number(value: &str) -> Result<f64, RuntimeError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(0.0);
    }
    if trimmed.eq_ignore_ascii_case("infinity") || trimmed == "+Infinity" {
        return Ok(f64::INFINITY);
    }
    if trimmed == "-Infinity" {
        return Ok(f64::NEG_INFINITY);
    }
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return Ok(u64::from_str_radix(hex, 16)
            .map(|value| value as f64)
            .unwrap_or(f64::NAN));
    }
    Ok(trimmed.parse::<f64>().unwrap_or(f64::NAN))
}

fn object_to_primitive(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    for method in ["valueOf", "toString"] {
        let method_value = property_value(value.clone(), method, env)?;
        if matches!(method_value, Value::Function(_)) {
            let primitive = call_function(method_value, value.clone(), Vec::new(), env, false)?;
            if !matches!(
                primitive,
                Value::Object(_) | Value::Function(_) | Value::Array(_)
            ) {
                return Ok(primitive);
            }
        }
    }
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: cannot convert object to primitive".to_owned(),
    })
}

fn object_to_number(value: Value, env: &mut HashMap<String, Value>) -> Result<f64, RuntimeError> {
    for method in ["valueOf", "toString"] {
        let method_value = property_value(value.clone(), method, env)?;
        if matches!(method_value, Value::Function(_)) {
            let primitive = call_function(method_value, value.clone(), Vec::new(), env, false)?;
            if !matches!(
                primitive,
                Value::Object(_) | Value::Function(_) | Value::Array(_)
            ) {
                return to_number_with_env(primitive, env);
            }
        }
    }
    Err(RuntimeError {
        thrown: None,
        message: "cannot convert object to number".to_owned(),
    })
}

fn object_to_string(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<String, RuntimeError> {
    for method in ["toString", "valueOf"] {
        let method_value = property_value(value.clone(), method, env)?;
        if matches!(method_value, Value::Function(_)) {
            let primitive = call_function(method_value, value.clone(), Vec::new(), env, false)?;
            if !matches!(
                primitive,
                Value::Object(_) | Value::Function(_) | Value::Array(_)
            ) {
                return to_js_string_with_env(primitive, env);
            }
        }
    }
    Err(RuntimeError {
        thrown: None,
        message: "cannot convert object to string".to_owned(),
    })
}

pub(crate) fn to_int32(value: Value) -> Result<i32, RuntimeError> {
    to_number(value).map(to_int32_number)
}

pub(crate) fn to_uint32(value: Value) -> Result<u32, RuntimeError> {
    to_number(value).map(to_uint32_number)
}

pub(crate) fn to_int32_number(number: f64) -> i32 {
    let int = to_uint32_number(number);
    if int >= 0x8000_0000 {
        (i64::from(int) - 0x1_0000_0000) as i32
    } else {
        int as i32
    }
}

pub(crate) fn to_uint32_number(number: f64) -> u32 {
    if !number.is_finite() || number == 0.0 {
        return 0;
    }
    const TWO_32: f64 = 4_294_967_296.0;
    number.trunc().rem_euclid(TWO_32) as u32
}

pub(crate) fn to_uint16(value: Value) -> Result<u16, RuntimeError> {
    let number = to_number(value)?;
    if !number.is_finite() || number == 0.0 {
        return Ok(0);
    }
    const TWO_16: f64 = 65_536.0;
    Ok(number.trunc().rem_euclid(TWO_16) as u16)
}

pub(crate) fn to_length(value: Value) -> Result<usize, RuntimeError> {
    let mut env = HashMap::new();
    to_length_with_env(value, &mut env)
}

pub(crate) fn to_length_with_env(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    const MAX_SAFE_INTEGER_LENGTH: usize = 9_007_199_254_740_991;
    let number = to_number_with_env(value, env)?;
    if number.is_nan() || number <= 0.0 {
        return Ok(0);
    }
    if number.is_infinite() {
        return Ok(MAX_SAFE_INTEGER_LENGTH);
    }
    Ok(number.trunc().min(MAX_SAFE_INTEGER_LENGTH as f64) as usize)
}

pub(crate) fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Number(number) => *number != 0.0 && !number.is_nan(),
        Value::String(value) => !value.is_empty(),
        Value::Boolean(value) => *value,
        Value::Null | Value::Undefined => false,
        Value::Function(_) | Value::Array(_) | Value::Object(_) => true,
    }
}
