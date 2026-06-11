use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    PropertyKey, RuntimeError, Value, array::array_join, call_function, date, number,
    property_value, property_value_key, symbol,
};

#[derive(Clone, Copy)]
pub(crate) enum PreferredType {
    Default,
    String,
    Number,
}

pub(crate) fn to_js_string(value: Value) -> Result<String, RuntimeError> {
    let mut env = crate::CallEnv::detached();
    to_js_string_with_env(value, &mut env)
}

pub(crate) fn to_js_string_with_env(
    value: Value,
    env: &mut CallEnv,
) -> Result<String, RuntimeError> {
    match value {
        Value::Number(number) => Ok(number::number_to_js_string(number)),
        Value::BigInt(value) => Ok(value.to_string()),
        Value::String(value) => Ok(value),
        Value::Boolean(true) => Ok("true".to_owned()),
        Value::Boolean(false) => Ok("false".to_owned()),
        Value::Null => Ok("null".to_owned()),
        Value::Undefined => Ok("undefined".to_owned()),
        Value::Object(object) if symbol::is_symbol_primitive(&object) => {
            Err(symbol_to_string_error())
        }
        Value::Object(object) => object_to_string(Value::Object(object), env),
        Value::Array(_) => array_join(value, ",", env),
        Value::Function(_) | Value::Map(_) | Value::Set(_) | Value::Proxy(_) => {
            object_to_string(value, env)
        }
    }
}

fn symbol_to_string_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: cannot convert Symbol to string".to_owned(),
    }
}

fn symbol_to_number_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: cannot convert Symbol to number".to_owned(),
    }
}

pub(crate) fn error_value(value: Value) -> String {
    match value {
        Value::Number(number) => number::number_to_js_string(number),
        Value::BigInt(value) => value.to_string(),
        Value::String(value) => value,
        Value::Boolean(true) => "true".to_owned(),
        Value::Boolean(false) => "false".to_owned(),
        Value::Null => "null".to_owned(),
        Value::Undefined => "undefined".to_owned(),
        Value::Function(_) => "function".to_owned(),
        Value::Array(_) => "array".to_owned(),
        Value::Map(_) | Value::Set(_) | Value::Proxy(_) => "object".to_owned(),
        Value::Object(object) => crate::error::error_object_to_string(&object)
            .or_else(|| object_constructor_name(&object))
            .unwrap_or_else(|| "object".to_owned()),
    }
}

fn object_constructor_name(object: &crate::ObjectRef) -> Option<String> {
    let Some(Value::Function(function)) = object.get("constructor") else {
        return None;
    };
    function.name.filter(|name| !name.is_empty())
}

pub(crate) fn to_number(value: Value) -> Result<f64, RuntimeError> {
    let mut env = crate::CallEnv::detached();
    to_number_with_env(value, &mut env)
}

pub(crate) fn to_number_with_env(value: Value, env: &mut CallEnv) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(number) => Ok(number),
        Value::BigInt(_) => Err(RuntimeError {
            thrown: None,
            message: "TypeError: cannot convert BigInt to number".to_owned(),
        }),
        Value::Boolean(true) => Ok(1.0),
        Value::Boolean(false) | Value::Null => Ok(0.0),
        Value::String(value) => string_to_number(&value),
        Value::Undefined => Ok(f64::NAN),
        Value::Object(object) if symbol::is_symbol_primitive(&object) => {
            Err(symbol_to_number_error())
        }
        Value::Object(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Array(_)
        | Value::Proxy(_) => object_to_number(value, env),
    }
}

pub(crate) fn to_primitive_with_env(
    value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    match value {
        Value::Object(object) if symbol::is_symbol_primitive(&object) => Ok(Value::Object(object)),
        Value::Object(_)
        | Value::Function(_)
        | Value::Array(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Proxy(_) => to_primitive_with_hint(value, PreferredType::Default, env),
        value => Ok(value),
    }
}

fn string_to_number(value: &str) -> Result<f64, RuntimeError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(0.0);
    }
    if trimmed == "Infinity" || trimmed == "+Infinity" {
        return Ok(f64::INFINITY);
    }
    if trimmed == "-Infinity" {
        return Ok(f64::NEG_INFINITY);
    }
    if trimmed
        .strip_prefix(['+', '-'])
        .unwrap_or(trimmed)
        .eq_ignore_ascii_case("infinity")
    {
        return Ok(f64::NAN);
    }
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return Ok(parse_radix_string_number(hex, 16));
    }
    if let Some(binary) = trimmed
        .strip_prefix("0b")
        .or_else(|| trimmed.strip_prefix("0B"))
    {
        return Ok(parse_radix_string_number(binary, 2));
    }
    if let Some(octal) = trimmed
        .strip_prefix("0o")
        .or_else(|| trimmed.strip_prefix("0O"))
    {
        return Ok(parse_radix_string_number(octal, 8));
    }
    Ok(trimmed.parse::<f64>().unwrap_or(f64::NAN))
}

fn parse_radix_string_number(digits: &str, radix: u32) -> f64 {
    if digits.is_empty() {
        return f64::NAN;
    }
    let mut value = 0.0;
    for digit in digits.chars() {
        let Some(digit) = digit.to_digit(radix) else {
            return f64::NAN;
        };
        value = value * f64::from(radix) + f64::from(digit);
    }
    value
}

pub(crate) fn to_primitive_with_hint(
    value: Value,
    hint: PreferredType,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if let Some(symbol) = symbol::to_primitive_symbol(env) {
        let method = property_value_key(value.clone(), &PropertyKey::Symbol(symbol), env)?;
        if !matches!(method, Value::Undefined | Value::Null) {
            let Value::Function(_) = method else {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: Symbol.toPrimitive method is not callable".to_owned(),
                });
            };
            let primitive = call_function(
                method,
                value.clone(),
                vec![Value::String(hint.name().to_owned())],
                env,
                false,
            )?;
            if !is_object_like(&primitive) {
                return Ok(primitive);
            }
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: Symbol.toPrimitive returned an object".to_owned(),
            });
        }
    }
    ordinary_to_primitive(value, hint, env)
}

pub(crate) fn ordinary_to_primitive(
    value: Value,
    hint: PreferredType,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let methods = match hint {
        PreferredType::String => ["toString", "valueOf"],
        PreferredType::Number => ["valueOf", "toString"],
        PreferredType::Default => match &value {
            Value::Object(object) if date::is_date_object(object) => ["toString", "valueOf"],
            _ => ["valueOf", "toString"],
        },
    };
    for method in methods {
        let method_value = property_value(value.clone(), method, env)?;
        if matches!(method_value, Value::Function(_)) {
            let primitive = call_function(method_value, value.clone(), Vec::new(), env, false)?;
            if !is_object_like(&primitive) {
                return Ok(primitive);
            }
        }
    }
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: cannot convert object to primitive".to_owned(),
    })
}

fn object_to_number(value: Value, env: &mut CallEnv) -> Result<f64, RuntimeError> {
    let primitive = to_primitive_with_hint(value, PreferredType::Number, env)?;
    to_number_with_env(primitive, env)
}

fn object_to_string(value: Value, env: &mut CallEnv) -> Result<String, RuntimeError> {
    let primitive = to_primitive_with_hint(value, PreferredType::String, env)?;
    to_js_string_with_env(primitive, env)
}

fn is_object_like(value: &Value) -> bool {
    match value {
        Value::Object(object) => !symbol::is_symbol_primitive(object),
        Value::Function(_) | Value::Array(_) | Value::Map(_) | Value::Set(_) => true,
        _ => false,
    }
}

impl PreferredType {
    fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::String => "string",
            Self::Number => "number",
        }
    }
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
    let mut env = crate::CallEnv::detached();
    to_length_with_env(value, &mut env)
}

pub(crate) fn to_length_with_env(value: Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
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
    if crate::html_dda::is_html_dda(value) {
        return false;
    }
    match value {
        Value::Number(number) => *number != 0.0 && !number.is_nan(),
        Value::BigInt(value) => value != &num_bigint::BigInt::from(0),
        Value::String(value) => !value.is_empty(),
        Value::Boolean(value) => *value,
        Value::Null | Value::Undefined => false,
        Value::Function(_)
        | Value::Array(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Object(_)
        | Value::Proxy(_) => true,
    }
}
