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
    if !number.is_finite() {
        return Ok(Value::String(number_to_js_string(number)));
    }
    let digits = fraction_digits(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::String(format!("{number:.digits$}")))
}

pub(crate) fn native_number_prototype_to_exponential(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let number = this_number_value(this_value)?;
    if !number.is_finite() {
        return Ok(Value::String(number_to_js_string(number)));
    }
    let digits = if matches!(argument_values.first(), None | Some(Value::Undefined)) {
        None
    } else {
        Some(fraction_digits(
            argument_values.first().cloned().unwrap_or(Value::Undefined),
        )?)
    };
    let output = match digits {
        Some(digits) => format!("{number:.digits$e}"),
        None => format!("{number:e}"),
    };
    Ok(Value::String(normalize_exponent(output)))
}

pub(crate) fn native_number_prototype_to_precision(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let number = this_number_value(this_value)?;
    let Some(precision_value) = argument_values.first().cloned() else {
        return Ok(Value::String(number_to_js_string(number)));
    };
    if matches!(precision_value, Value::Undefined) {
        return Ok(Value::String(number_to_js_string(number)));
    }
    if !number.is_finite() {
        return Ok(Value::String(number_to_js_string(number)));
    }
    let precision = precision_digits(precision_value)?;
    Ok(Value::String(to_precision_string(number, precision)))
}

pub(crate) fn native_number_prototype_value_of(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::Number(this_number_value(this_value)?))
}

fn fraction_digits(value: Value) -> Result<usize, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(0);
    }
    let digits = to_int32(value)?;
    if !(0..=100).contains(&digits) {
        return Err(RuntimeError {
            thrown: None,
            message: "fraction digits must be between 0 and 100".to_owned(),
        });
    }
    Ok(digits as usize)
}

fn precision_digits(value: Value) -> Result<usize, RuntimeError> {
    let digits = to_int32(value)?;
    if !(1..=100).contains(&digits) {
        return Err(RuntimeError {
            thrown: None,
            message: "precision must be between 1 and 100".to_owned(),
        });
    }
    Ok(digits as usize)
}

fn normalize_exponent(output: String) -> String {
    if let Some((mantissa, exponent)) = output.split_once('e') {
        let sign = if exponent.starts_with('-') { "" } else { "+" };
        format!("{mantissa}e{sign}{exponent}")
    } else {
        output
    }
}

fn to_precision_string(number: f64, precision: usize) -> String {
    if number == 0.0 {
        if precision == 1 {
            return "0".to_owned();
        }
        return format!("0.{}", "0".repeat(precision - 1));
    }

    let exponent = number.abs().log10().floor() as i32;
    if exponent < -6 || exponent >= precision as i32 {
        return normalize_exponent(format!("{number:.digits$e}", digits = precision - 1));
    }

    let fraction_digits = (precision as i32 - exponent - 1).max(0) as usize;
    format!("{number:.fraction_digits$}")
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
