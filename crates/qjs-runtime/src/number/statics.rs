use crate::{RuntimeError, Value};

pub(crate) fn native_number_is_finite(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Number(number)) if number.is_finite()
    )))
}

pub(crate) fn native_number_is_integer(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Number(number)) if number.is_finite() && number.fract() == 0.0
    )))
}

pub(crate) fn native_number_is_nan(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Number(number)) if number.is_nan()
    )))
}

pub(crate) fn native_number_is_safe_integer(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Number(number))
            if number.is_finite() && number.fract() == 0.0 && number.abs() <= MAX_SAFE_INTEGER
    )))
}
