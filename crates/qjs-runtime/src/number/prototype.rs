use std::collections::HashMap;

use num_traits::ToPrimitive;

use crate::{
    Function, ObjectRef, Property, RuntimeError, Value, function_prototype, to_int32, to_number,
    to_number_with_env,
};

use super::{NUMBER_DATA_PROPERTY, formatting::number_to_js_string};

pub(crate) fn native_number(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
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
) -> Result<Value, RuntimeError> {
    let number = this_number_value(this_value)?;
    let fraction_digits =
        fraction_digits(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::String(number_to_fixed_string(
        number,
        fraction_digits,
    )))
}

pub(crate) fn native_number_prototype_to_exponential(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let number = this_number_value(this_value)?;
    if !number.is_finite() {
        return Ok(Value::String(number_to_js_string(number)));
    }
    let fraction_digits = optional_digits(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        0,
        100,
        "RangeError: toExponential fraction digits must be between 0 and 100",
    )?;
    Ok(Value::String(number_to_exponential_string(
        number,
        fraction_digits,
    )))
}

pub(crate) fn native_number_prototype_to_precision(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let number = this_number_value(this_value)?;
    if !number.is_finite() {
        return Ok(Value::String(number_to_js_string(number)));
    }
    let precision = optional_digits(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        1,
        100,
        "RangeError: toPrecision precision must be between 1 and 100",
    )?;
    Ok(Value::String(number_to_precision_string(number, precision)))
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

fn fraction_digits(value: Value) -> Result<usize, RuntimeError> {
    match optional_digits(
        value,
        0,
        100,
        "RangeError: toFixed fraction digits must be between 0 and 100",
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
) -> Result<Option<usize>, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(None);
    }
    let digits = to_number(value)?;
    let digits = if digits.is_nan() { 0.0 } else { digits.trunc() };
    if digits < min as f64 || digits > max as f64 {
        return Err(RuntimeError {
            thrown: None,
            message: range_message.to_owned(),
        });
    }
    Ok(Some(digits as usize))
}

fn number_to_fixed_string(number: f64, fraction_digits: usize) -> String {
    if !number.is_finite() || number.abs() >= 1e21 {
        return number_to_js_string(number);
    }
    let number = if number == 0.0 { 0.0 } else { number };
    format!("{number:.fraction_digits$}")
}

fn number_to_exponential_string(number: f64, fraction_digits: Option<usize>) -> String {
    if !number.is_finite() {
        return number_to_js_string(number);
    }
    let number = if number == 0.0 { 0.0 } else { number };
    let formatted = match fraction_digits {
        Some(fraction_digits) => format!("{number:.fraction_digits$e}"),
        None => format!("{number:e}"),
    };
    normalize_exponential_string(&formatted)
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
