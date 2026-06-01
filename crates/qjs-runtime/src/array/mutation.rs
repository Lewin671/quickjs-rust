use crate::{RuntimeError, Value};

use super::indexing::{array_slice_end, array_slice_start};

pub(crate) fn native_array_prototype_fill(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value.clone() else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.fill called on non-array".to_owned(),
        });
    };

    let length = elements.len();
    let start = array_slice_start(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    let end = array_slice_end(
        length,
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
    )?;
    if start < end {
        elements.fill(
            start,
            end,
            argument_values.first().cloned().unwrap_or(Value::Undefined),
        );
    }
    Ok(this_value)
}

pub(crate) fn native_array_prototype_copy_within(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value.clone() else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.copyWithin called on non-array".to_owned(),
        });
    };

    let length = elements.len();
    let target = array_slice_start(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
    )?;
    let start = array_slice_start(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    let end = array_slice_end(
        length,
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
    )?;
    let count = (end.saturating_sub(start)).min(length.saturating_sub(target));
    if count == 0 {
        return Ok(this_value);
    }

    let snapshot = elements.to_vec();
    for offset in 0..count {
        elements.set(target + offset, snapshot[start + offset].clone());
    }
    Ok(this_value)
}

pub(crate) fn native_array_prototype_push(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.push called on non-array".to_owned(),
        });
    };
    for value in argument_values.iter().cloned() {
        elements.push(value);
    }
    Ok(Value::Number(elements.len() as f64))
}

pub(crate) fn native_array_prototype_pop(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.pop called on non-array".to_owned(),
        });
    };
    Ok(elements.pop().unwrap_or(Value::Undefined))
}

pub(crate) fn native_array_prototype_shift(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.shift called on non-array".to_owned(),
        });
    };
    Ok(elements.shift().unwrap_or(Value::Undefined))
}

pub(crate) fn native_array_prototype_unshift(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.unshift called on non-array".to_owned(),
        });
    };
    Ok(Value::Number(elements.unshift(argument_values) as f64))
}

pub(crate) fn native_array_prototype_reverse(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value.clone() else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.reverse called on non-array".to_owned(),
        });
    };
    elements.reverse();
    Ok(this_value)
}
