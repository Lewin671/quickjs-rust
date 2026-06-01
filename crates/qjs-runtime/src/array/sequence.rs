use crate::{ArrayRef, RuntimeError, Value};

use super::array_like::array_like_values;
use super::indexing::{array_at_index, array_slice_end, array_slice_start};
use super::splice::{splice_delete_count, splice_start};

pub(crate) fn native_array_prototype_concat(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let mut result = Vec::new();
    concat_array_item(&mut result, this_value);
    for value in argument_values.iter().cloned() {
        concat_array_item(&mut result, value);
    }
    Ok(Value::Array(ArrayRef::new(result)))
}

pub(crate) fn native_array_prototype_slice(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.slice called on non-array".to_owned(),
        });
    };

    let length = elements.len();
    let start = array_slice_start(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
    )?;
    let end = array_slice_end(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;

    if end <= start {
        return Ok(Value::Array(ArrayRef::new(Vec::new())));
    }
    Ok(Value::Array(ArrayRef::new(
        elements.to_vec()[start..end].to_vec(),
    )))
}

pub(crate) fn native_array_prototype_to_reversed(this_value: Value) -> Result<Value, RuntimeError> {
    let mut values = array_like_values(this_value, "Array.prototype.toReversed")?;
    values.reverse();
    Ok(Value::Array(ArrayRef::new(values)))
}

pub(crate) fn native_array_prototype_to_spliced(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let values = array_like_values(this_value, "Array.prototype.toSpliced")?;
    let length = values.len();
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

    let mut result = Vec::with_capacity(length.saturating_sub(delete_count) + items.len());
    result.extend_from_slice(&values[..start]);
    result.extend_from_slice(items);
    result.extend_from_slice(&values[start + delete_count..]);
    Ok(Value::Array(ArrayRef::new(result)))
}

pub(crate) fn native_array_prototype_with(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let mut values = array_like_values(this_value, "Array.prototype.with")?;
    let index = array_at_index(
        values.len(),
        argument_values.first().cloned().unwrap_or(Value::Undefined),
    )?
    .ok_or_else(|| RuntimeError {
        thrown: None,
        message: "Array.prototype.with index out of range".to_owned(),
    })?;
    values[index] = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    Ok(Value::Array(ArrayRef::new(values)))
}

fn concat_array_item(result: &mut Vec<Value>, value: Value) {
    match value {
        Value::Array(elements) => result.extend(elements.to_vec()),
        value => result.push(value),
    }
}
