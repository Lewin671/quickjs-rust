use crate::{RuntimeError, Value, number};

pub(crate) fn to_js_string(value: Value) -> Result<String, RuntimeError> {
    match value {
        Value::Number(number) => Ok(number::number_to_js_string(number)),
        Value::String(value) => Ok(value),
        Value::Boolean(true) => Ok("true".to_owned()),
        Value::Boolean(false) => Ok("false".to_owned()),
        Value::Null => Ok("null".to_owned()),
        Value::Undefined => Ok("undefined".to_owned()),
        Value::Object(object) => crate::string_object_value(&object).ok_or_else(|| RuntimeError {
            message: "cannot convert object to string".to_owned(),
        }),
        Value::Function(_) | Value::Array(_) => Err(RuntimeError {
            message: "cannot convert object to string".to_owned(),
        }),
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
        Value::Object(_) => "object".to_owned(),
    }
}

pub(crate) fn to_number(value: Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(number) => Ok(number),
        Value::Boolean(true) => Ok(1.0),
        Value::Boolean(false) | Value::Null => Ok(0.0),
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Ok(0.0)
            } else {
                Ok(trimmed.parse::<f64>().unwrap_or(f64::NAN))
            }
        }
        Value::Undefined => Ok(f64::NAN),
        Value::Object(object) => match crate::string_object_value(&object) {
            Some(value) => to_number(Value::String(value)),
            None => Err(RuntimeError {
                message: "cannot convert object to number".to_owned(),
            }),
        },
        Value::Function(_) => Err(RuntimeError {
            message: "cannot convert function to number".to_owned(),
        }),
        Value::Array(_) => Err(RuntimeError {
            message: "cannot convert object to number".to_owned(),
        }),
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
    let number = to_number(value)?;
    if number.is_nan() || number <= 0.0 {
        return Ok(0);
    }
    if number.is_infinite() {
        return Err(RuntimeError {
            message: "string padding length must be finite".to_owned(),
        });
    }
    Ok(number.trunc().min(9_007_199_254_740_991.0) as usize)
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
