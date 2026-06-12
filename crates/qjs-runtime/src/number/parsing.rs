use crate::{CallEnv, RuntimeError, Value, to_int32_with_env, to_js_string_with_env};

pub(crate) fn native_parse_float(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let input = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    Ok(Value::Number(parse_float_string(&input)))
}

fn parse_float_string(input: &str) -> f64 {
    let input = input.trim_start();
    if input.starts_with("Infinity") {
        return f64::INFINITY;
    }
    if input.starts_with("+Infinity") {
        return f64::INFINITY;
    }
    if input.starts_with("-Infinity") {
        return f64::NEG_INFINITY;
    }

    let bytes = input.as_bytes();
    let mut end = 0;
    if matches!(bytes.first(), Some(b'+') | Some(b'-')) {
        end = 1;
    }

    let mut digits_before_dot = 0usize;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        digits_before_dot += 1;
        end += 1;
    }

    let mut digits_after_dot = 0usize;
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            digits_after_dot += 1;
            end += 1;
        }
    }

    if digits_before_dot + digits_after_dot == 0 {
        return f64::NAN;
    }

    let exponent_marker = end;
    if matches!(bytes.get(end), Some(b'e') | Some(b'E')) {
        let mut exponent_end = end + 1;
        if matches!(bytes.get(exponent_end), Some(b'+') | Some(b'-')) {
            exponent_end += 1;
        }
        let exponent_digits_start = exponent_end;
        while bytes.get(exponent_end).is_some_and(u8::is_ascii_digit) {
            exponent_end += 1;
        }
        if exponent_end > exponent_digits_start {
            end = exponent_end;
        } else {
            end = exponent_marker;
        }
    }

    input[..end].parse::<f64>().unwrap_or(f64::NAN)
}

pub(crate) fn native_parse_int(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let input = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let radix = argument_values
        .get(1)
        .cloned()
        .map(|value| to_int32_with_env(value, env))
        .transpose()?
        .unwrap_or(0);
    Ok(Value::Number(parse_int_string(&input, radix)))
}

fn parse_int_string(input: &str, radix: i32) -> f64 {
    let mut input = input.trim_start();
    let mut sign = 1.0;
    if let Some(rest) = input.strip_prefix('-') {
        sign = -1.0;
        input = rest;
    } else if let Some(rest) = input.strip_prefix('+') {
        input = rest;
    }

    let mut radix = radix;
    if radix != 0 && !(2..=36).contains(&radix) {
        return f64::NAN;
    }

    if radix == 0 {
        if let Some(rest) = input
            .strip_prefix("0x")
            .or_else(|| input.strip_prefix("0X"))
        {
            input = rest;
            radix = 16;
        } else {
            radix = 10;
        }
    } else if radix == 16
        && let Some(rest) = input
            .strip_prefix("0x")
            .or_else(|| input.strip_prefix("0X"))
    {
        input = rest;
    }

    let radix = radix as u32;
    let mut value = 0.0;
    let mut digits = 0usize;
    for character in input.chars() {
        let Some(digit) = character.to_digit(36) else {
            break;
        };
        if digit >= radix {
            break;
        }
        value = value * f64::from(radix) + f64::from(digit);
        digits += 1;
    }

    if digits == 0 { f64::NAN } else { sign * value }
}
