use std::collections::HashMap;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::{
    Function, ObjectRef, Property, RuntimeError, Value, function_prototype, to_int32,
    to_number_with_env,
};

use super::{NUMBER_DATA_PROPERTY, formatting::number_to_js_string};
use crate::CallEnv;

pub(crate) fn native_number(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let number = match argument_values.first() {
        Some(Value::BigInt(value)) => value.to_f64().unwrap_or_else(|| {
            if value.sign() == num_bigint::Sign::Minus {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            }
        }),
        Some(value) => to_number_with_env(value.clone(), env)?,
        None => 0.0,
    };
    if !is_construct {
        return Ok(Value::Number(number));
    }

    let object = match this_value {
        Value::Object(object) => object,
        _ => ObjectRef::with_prototype(HashMap::new(), function_prototype(function)),
    };
    object.define_non_enumerable(NUMBER_DATA_PROPERTY.to_owned(), Value::Number(number));
    Ok(Value::Object(object))
}

pub(crate) fn native_number_prototype_to_string(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let number = this_number_value(this_value)?;
    let radix =
        number_to_string_radix(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::String(number_to_radix_string(number, radix)?))
}

pub(crate) fn native_number_prototype_to_fixed(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let number = this_number_value(this_value)?;
    let fraction_digits = fraction_digits(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    Ok(Value::String(number_to_fixed_string(
        number,
        fraction_digits,
    )))
}

pub(crate) fn native_number_prototype_to_exponential(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let number = this_number_value(this_value)?;
    let fraction_digits = optional_digit_number(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if !number.is_finite() {
        return Ok(Value::String(number_to_js_string(number)));
    }
    let fraction_digits = match fraction_digits {
        Some(digits) => Some(validate_digits(
            digits,
            0,
            100,
            "RangeError: toExponential fraction digits must be between 0 and 100",
        )?),
        None => None,
    };
    Ok(Value::String(number_to_exponential_string(
        number,
        fraction_digits,
    )))
}

pub(crate) fn native_number_prototype_to_precision(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let number = this_number_value(this_value)?;
    if matches!(argument_values.first(), None | Some(Value::Undefined)) {
        return Ok(Value::String(number_to_js_string(number)));
    }
    let precision = optional_digit_number(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if !number.is_finite() {
        return Ok(Value::String(number_to_js_string(number)));
    }
    let precision = validate_digits(
        precision.expect("precision undefined handled above"),
        1,
        100,
        "RangeError: toPrecision precision must be between 1 and 100",
    )?;
    Ok(Value::String(number_to_precision_string(
        number,
        Some(precision),
    )))
}

pub(crate) fn native_number_prototype_value_of(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::Number(this_number_value(this_value)?))
}

fn this_number_value(value: Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(value) => Ok(value),
        Value::Object(object) => match object.own_property(NUMBER_DATA_PROPERTY) {
            Some(Property {
                value: Value::Number(value),
                ..
            }) => Ok(value),
            _ => Err(RuntimeError {
                thrown: None,
                message: "Number.prototype method called on non-number object".to_owned(),
            }),
        },
        _ => Err(RuntimeError {
            thrown: None,
            message: "Number.prototype method called on non-number".to_owned(),
        }),
    }
}

fn number_to_string_radix(value: Value) -> Result<u32, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(10);
    }
    let radix = to_int32(value)?;
    if !(2..=36).contains(&radix) {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: radix must be between 2 and 36".to_owned(),
        });
    }
    Ok(radix as u32)
}

fn fraction_digits(value: Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
    match optional_digits(
        value,
        0,
        100,
        "RangeError: toFixed fraction digits must be between 0 and 100",
        env,
    )? {
        Some(digits) => Ok(digits),
        None => Ok(0),
    }
}

fn optional_digits(
    value: Value,
    min: usize,
    max: usize,
    range_message: &str,
    env: &mut CallEnv,
) -> Result<Option<usize>, RuntimeError> {
    let Some(digits) = optional_digit_number(value, env)? else {
        return Ok(None);
    };
    Ok(Some(validate_digits(digits, min, max, range_message)?))
}

fn optional_digit_number(value: Value, env: &mut CallEnv) -> Result<Option<f64>, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(None);
    }
    let digits = to_number_with_env(value, env)?;
    Ok(Some(if digits.is_nan() { 0.0 } else { digits.trunc() }))
}

fn validate_digits(
    digits: f64,
    min: usize,
    max: usize,
    range_message: &str,
) -> Result<usize, RuntimeError> {
    if digits < min as f64 || digits > max as f64 {
        return Err(RuntimeError {
            thrown: None,
            message: range_message.to_owned(),
        });
    }
    Ok(digits as usize)
}

fn number_to_fixed_string(number: f64, fraction_digits: usize) -> String {
    if !number.is_finite() || number.abs() >= 1e21 {
        return number_to_js_string(number);
    }
    let sign = if number < 0.0 { "-" } else { "" };
    let digits = rounded_scaled_power10(number.abs(), fraction_digits as i32).to_string();
    if fraction_digits == 0 {
        return format!("{sign}{digits}");
    }
    if digits.len() <= fraction_digits {
        return format!(
            "{sign}0.{}{}",
            "0".repeat(fraction_digits - digits.len()),
            digits
        );
    }
    let decimal_position = digits.len() - fraction_digits;
    format!(
        "{sign}{}.{}",
        &digits[..decimal_position],
        &digits[decimal_position..]
    )
}

fn rounded_scaled_power10(number: f64, decimal_exponent: i32) -> BigInt {
    let (significand, exponent) = positive_f64_parts(number);
    let mut numerator = BigInt::from(significand);
    let mut denominator = BigInt::from(1_u8);
    if exponent >= 0 {
        numerator <<= exponent as usize;
    } else {
        denominator <<= (-exponent) as usize;
    }
    if decimal_exponent >= 0 {
        numerator *= ten_pow(decimal_exponent as usize);
    } else {
        denominator *= ten_pow((-decimal_exponent) as usize);
    }

    let quotient = &numerator / &denominator;
    let remainder = numerator % &denominator;
    if (remainder << 1_usize) >= denominator {
        quotient + 1
    } else {
        quotient
    }
}

fn positive_f64_parts(number: f64) -> (u64, i32) {
    if number == 0.0 {
        return (0, 0);
    }
    let bits = number.to_bits();
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let fraction_bits = bits & ((1_u64 << 52) - 1);
    if exponent_bits == 0 {
        (fraction_bits, -1074)
    } else {
        ((1_u64 << 52) | fraction_bits, exponent_bits - 1075)
    }
}

fn ten_pow(exponent: usize) -> BigInt {
    let mut value = BigInt::from(1_u8);
    for _ in 0..exponent {
        value *= 10_u8;
    }
    value
}

fn number_to_exponential_string(number: f64, fraction_digits: Option<usize>) -> String {
    if !number.is_finite() {
        return number_to_js_string(number);
    }
    if let Some(fraction_digits) = fraction_digits {
        return number_to_exact_exponential_string(number, fraction_digits);
    }
    let number = if number == 0.0 { 0.0 } else { number };
    let formatted = format!("{number:e}");
    normalize_exponential_string(&formatted)
}

fn number_to_exact_exponential_string(number: f64, fraction_digits: usize) -> String {
    if number == 0.0 {
        return format!("0{}e+0", fractional_zero_suffix(fraction_digits));
    }

    let sign = if number < 0.0 { "-" } else { "" };
    let mut exponent = number.abs().log10().floor() as i32;
    let precision = fraction_digits + 1;
    let mut digits = rounded_scaled_power10(number.abs(), fraction_digits as i32 - exponent);
    let precision_limit = ten_pow(precision);
    if digits >= precision_limit {
        digits /= 10_u8;
        exponent += 1;
    }

    let mut digits = digits.to_string();
    if digits.len() < precision {
        digits = format!("{}{}", "0".repeat(precision - digits.len()), digits);
    }
    let exponent_sign = if exponent < 0 { '-' } else { '+' };
    if fraction_digits == 0 {
        return format!("{sign}{digits}e{exponent_sign}{}", exponent.abs());
    }
    format!(
        "{sign}{}.{}e{exponent_sign}{}",
        &digits[..1],
        &digits[1..],
        exponent.abs()
    )
}

fn fractional_zero_suffix(fraction_digits: usize) -> String {
    if fraction_digits == 0 {
        String::new()
    } else {
        format!(".{}", "0".repeat(fraction_digits))
    }
}

fn number_to_precision_string(number: f64, precision: Option<usize>) -> String {
    if !number.is_finite() || precision.is_none() {
        return number_to_js_string(number);
    }
    let precision = precision.expect("precision checked above");
    let number = if number == 0.0 { 0.0 } else { number };
    let formatted = format!(
        "{number:.fraction_digits$e}",
        fraction_digits = precision - 1
    );
    let (sign, digits, exponent) = exponential_parts(&formatted);
    if exponent < -6 || exponent >= precision as i32 {
        return normalize_exponential_string(&formatted);
    }

    let decimal_position = exponent + 1;
    if decimal_position <= 0 {
        return format!(
            "{sign}0.{}{}",
            "0".repeat((-decimal_position) as usize),
            digits
        );
    }
    let decimal_position = decimal_position as usize;
    if decimal_position >= digits.len() {
        return format!(
            "{sign}{}{}",
            digits,
            "0".repeat(decimal_position - digits.len())
        );
    }
    format!(
        "{sign}{}.{}",
        &digits[..decimal_position],
        &digits[decimal_position..]
    )
}

fn normalize_exponential_string(value: &str) -> String {
    let Some((mantissa, exponent)) = value.split_once('e') else {
        return value.to_owned();
    };
    let exponent = normalize_exponent(exponent);
    format!("{mantissa}e{exponent}")
}

fn exponential_parts(value: &str) -> (&str, String, i32) {
    let Some((mantissa, exponent)) = value.split_once('e') else {
        return ("", value.to_owned(), 0);
    };
    let (sign, mantissa) = mantissa
        .strip_prefix('-')
        .map_or(("", mantissa), |mantissa| ("-", mantissa));
    let digits = mantissa.replace('.', "");
    let exponent = exponent.parse::<i32>().unwrap_or(0);
    (sign, digits, exponent)
}

fn normalize_exponent(exponent: &str) -> String {
    if let Some(unsigned) = exponent.strip_prefix('-') {
        let trimmed = unsigned.trim_start_matches('0');
        format!("-{}", if trimmed.is_empty() { "0" } else { trimmed })
    } else {
        let trimmed = exponent.trim_start_matches('0');
        format!("+{}", if trimmed.is_empty() { "0" } else { trimmed })
    }
}

fn number_to_radix_string(number: f64, radix: u32) -> Result<String, RuntimeError> {
    if radix == 10 || !number.is_finite() {
        return Ok(number_to_js_string(number));
    }
    if number.fract() != 0.0 {
        return Err(RuntimeError {
            thrown: None,
            message: "non-decimal number formatting supports integers only".to_owned(),
        });
    }

    let sign = if number < 0.0 { "-" } else { "" };
    let mut integer = number.abs() as u128;
    if integer == 0 {
        return Ok("0".to_owned());
    }

    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut output = Vec::new();
    while integer > 0 {
        let digit = (integer % u128::from(radix)) as usize;
        output.push(DIGITS[digit] as char);
        integer /= u128::from(radix);
    }
    output.reverse();
    Ok(format!("{sign}{}", output.into_iter().collect::<String>()))
}
