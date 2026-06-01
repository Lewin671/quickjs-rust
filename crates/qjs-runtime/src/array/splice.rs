use crate::{ArrayRef, RuntimeError, Value, to_number};

pub(crate) fn native_array_prototype_splice(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(array) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.splice called on non-array".to_owned(),
        });
    };

    let length = array.len();
    let start = splice_start(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
    )?;
    let delete_count = splice_delete_count(length, start, argument_values)?;
    let items = if argument_values.len() > 2 {
        &argument_values[2..]
    } else {
        &[]
    };

    let removed = array.splice(start, delete_count, items);
    Ok(Value::Array(ArrayRef::new(removed)))
}

pub(super) fn splice_start(length: usize, start: Value) -> Result<usize, RuntimeError> {
    let number = match start {
        Value::Undefined => 0.0,
        value => to_number(value)?,
    };
    if number.is_nan() {
        return Ok(0);
    }

    let integer = number.trunc();
    if integer < 0.0 {
        Ok((length as f64 + integer).max(0.0) as usize)
    } else {
        Ok(integer.min(length as f64) as usize)
    }
}

pub(super) fn splice_delete_count(
    length: usize,
    start: usize,
    argument_values: &[Value],
) -> Result<usize, RuntimeError> {
    if argument_values.len() < 2 {
        return Ok(length.saturating_sub(start));
    }

    let number = to_number(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    if number.is_nan() || number <= 0.0 {
        return Ok(0);
    }
    Ok((number.trunc() as usize).min(length.saturating_sub(start)))
}
