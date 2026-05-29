use crate::{ArrayRef, RuntimeError, Value, to_number};

pub(crate) fn native_array_prototype_flat(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(array) = this_value else {
        return Err(RuntimeError {
            message: "Array.prototype.flat called on non-array".to_owned(),
        });
    };

    let depth = flat_depth(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let mut result = Vec::new();
    flatten_into(&mut result, array.to_vec(), depth);
    Ok(Value::Array(ArrayRef::new(result)))
}

fn flat_depth(value: Value) -> Result<usize, RuntimeError> {
    let number = match value {
        Value::Undefined => return Ok(1),
        value => to_number(value)?,
    };

    if number.is_nan() || number <= 0.0 {
        return Ok(0);
    }
    if number.is_infinite() {
        return Ok(usize::MAX);
    }
    Ok(number.trunc() as usize)
}

fn flatten_into(result: &mut Vec<Value>, values: Vec<Value>, depth: usize) {
    for value in values {
        match value {
            Value::Array(array) if depth > 0 => {
                flatten_into(result, array.to_vec(), depth.saturating_sub(1));
            }
            value => result.push(value),
        }
    }
}
