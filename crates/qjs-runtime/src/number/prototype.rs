use std::collections::HashMap;

use crate::{
    Function, ObjectRef, Property, RuntimeError, Value, function_prototype, to_int32, to_number,
};

use super::{NUMBER_DATA_PROPERTY, formatting::number_to_js_string};

pub(crate) fn native_number(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    let number = match argument_values.first() {
        Some(value) => to_number(value.clone())?,
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
            message: "radix must be between 2 and 36".to_owned(),
        });
    }
    Ok(radix as u32)
}

fn fraction_digits(value: Value) -> Result<usize, RuntimeError> {
    let digits = to_number(value)?;
    let digits = if digits.is_nan() { 0.0 } else { digits.trunc() };
    if !(0.0..=100.0).contains(&digits) {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: toFixed fraction digits must be between 0 and 100".to_owned(),
        });
    }
    Ok(digits as usize)
}

fn number_to_fixed_string(number: f64, fraction_digits: usize) -> String {
    if !number.is_finite() || number.abs() >= 1e21 {
        return number_to_js_string(number);
    }
    let number = if number == 0.0 { 0.0 } else { number };
    format!("{number:.fraction_digits$}")
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
