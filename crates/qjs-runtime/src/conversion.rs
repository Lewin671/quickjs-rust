use crate::CallEnv;
use crate::{
    PropertyKey, RuntimeError, Value, call_function, date, number, property_value,
    property_value_key, symbol,
};

#[derive(Clone, Copy)]
pub(crate) enum PreferredType {
    Default,
    String,
    Number,
}

pub(crate) fn to_js_string_with_env(
    value: Value,
    env: &mut CallEnv,
) -> Result<String, RuntimeError> {
    match value {
        Value::Number(number) => Ok(number::number_to_js_string(number)),
        Value::BigInt(value) => Ok(value.to_string()),
        Value::String(value) => Ok(std::rc::Rc::unwrap_or_clone(value)),
        Value::Boolean(true) => Ok("true".to_owned()),
        Value::Boolean(false) => Ok("false".to_owned()),
        Value::Null => Ok("null".to_owned()),
        Value::Undefined => Ok("undefined".to_owned()),
        Value::Object(object) if symbol::is_symbol_primitive(&object) => {
            Err(symbol_to_string_error())
        }
        Value::Object(object) => object_to_string(Value::Object(object), env),
        // Route arrays through ToPrimitive so an overridden `toString` (or
        // `Array.prototype.toString`) is honored; the default still joins.
        Value::Array(_) => object_to_string(value, env),
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
        Value::String(value) => value.to_string(),
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
    function.name.clone().filter(|name| !name.is_empty())
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

pub(crate) fn string_to_number(value: &str) -> Result<f64, RuntimeError> {
    let trimmed = value.trim_matches(is_ecmascript_trim_code_point);
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
    if let Some(number) = long_zero_fraction_number(trimmed) {
        return Ok(number);
    }
    Ok(trimmed.parse::<f64>().unwrap_or(f64::NAN))
}

fn long_zero_fraction_number(trimmed: &str) -> Option<f64> {
    let (sign, unsigned) = match trimmed.as_bytes().first() {
        Some(b'+') => (1.0, &trimmed[1..]),
        Some(b'-') => (-1.0, &trimmed[1..]),
        _ => (1.0, trimmed),
    };
    let (integer, fraction) = unsigned.split_once('.')?;
    if integer.is_empty()
        || fraction.len() < 20
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || !integer.bytes().any(|byte| byte != b'0')
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.as_bytes()[..20].iter().all(|byte| *byte == b'0')
    {
        return None;
    }
    Some(sign * integer.parse::<f64>().ok()?)
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
    // ToPrimitive step 1: a non-Object input is returned unchanged. (Symbol
    // primitives are stored as objects but excluded by `is_object_like`, so
    // they pass through here and are rejected later by ToNumber, per spec.)
    if !is_object_like(&value) {
        return Ok(value);
    }
    if let Some(symbol) = symbol::to_primitive_symbol(env) {
        let method = property_value_key(value.clone(), &PropertyKey::Symbol(symbol), env)?;
        if !matches!(method, Value::Undefined | Value::Null) {
            if !is_callable(&method) {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: Symbol.toPrimitive method is not callable".to_owned(),
                });
            }
            let primitive = call_function(
                method,
                value.clone(),
                vec![Value::String(hint.name().to_owned().into())],
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
        if is_callable(&method_value) {
            let mut method_env = to_primitive_method_env(&method_value, env);
            let primitive = call_function(
                method_value,
                value.clone(),
                Vec::new(),
                &mut method_env,
                false,
            );
            sync_to_primitive_method_env(env, &method_env);
            let primitive = primitive?;
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
        Value::Function(_) | Value::Array(_) | Value::Map(_) | Value::Set(_) | Value::Proxy(_) => {
            true
        }
        _ => false,
    }
}

fn sync_to_primitive_method_env(env: &mut CallEnv, method_env: &CallEnv) {
    for (name, value) in method_env.binding_snapshot() {
        if env.set_local(&name, value.clone()) {
        } else if env.realm_contains(&name) {
            env.insert_realm(name, value);
        }
    }
}

fn to_primitive_method_env(method: &Value, env: &CallEnv) -> CallEnv {
    let mut method_env = env.clone();
    if let Value::Function(function) = method {
        let mut captured_names = function.native_context.keys().cloned().collect::<Vec<_>>();
        if let Some(bytecode) = &function.bytecode {
            captured_names.extend(bytecode.received_upvalue_names().map(str::to_owned));
        }
        captured_names.sort();
        captured_names.dedup();
        for name in captured_names {
            if function
                .bytecode
                .as_ref()
                .is_some_and(|bytecode| bytecode.writes_binding(&name))
            {
                continue;
            }
            let captured_value = function
                .bytecode
                .as_ref()
                .and_then(|bytecode| {
                    bytecode
                        .received_upvalue_names()
                        .zip(&function.upvalues)
                        .find_map(|(candidate, upvalue)| (candidate == name).then(|| upvalue.get()))
                })
                .or_else(|| function.native_context.get(&name).cloned());
            if env.get_local(&name) != captured_value {
                method_env.remove(&name);
            }
        }
    }
    method_env
}

fn is_callable(value: &Value) -> bool {
    match value {
        Value::Function(_) => true,
        Value::Proxy(proxy) => crate::proxy::proxy_is_callable(proxy),
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

pub(crate) fn to_int32_with_env(value: Value, env: &mut CallEnv) -> Result<i32, RuntimeError> {
    to_number_with_env(value, env).map(to_int32_number)
}

pub(crate) fn to_uint32_with_env(value: Value, env: &mut CallEnv) -> Result<u32, RuntimeError> {
    to_number_with_env(value, env).map(to_uint32_number)
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

pub(crate) fn to_uint16_with_env(value: Value, env: &mut CallEnv) -> Result<u16, RuntimeError> {
    let number = to_number_with_env(value, env)?;
    if !number.is_finite() || number == 0.0 {
        return Ok(0);
    }
    const TWO_16: f64 = 65_536.0;
    Ok(number.trunc().rem_euclid(TWO_16) as u16)
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
        Value::BigInt(value) => **value != num_bigint::BigInt::from(0),
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

#[cfg(test)]
mod tests {
    use super::string_to_number;

    #[test]
    fn string_to_number_fast_paths_long_zero_fraction() {
        assert_eq!(
            string_to_number("17.0000000000000000000000000000000000000000001"),
            Ok(17.0)
        );
        assert_eq!(
            string_to_number("-17.0000000000000000000000000000000000000000001"),
            Ok(-17.0)
        );
        assert_eq!(
            string_to_number("0.000000000000000000001"),
            Ok(0.000000000000000000001)
        );
    }
}
