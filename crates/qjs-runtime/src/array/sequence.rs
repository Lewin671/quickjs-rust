use std::collections::HashMap;

use crate::{
    ArrayRef, PropertyKey, RuntimeError, Value, has_property, is_truthy, property_value,
    property_value_key, symbol, to_length_with_env,
};

use super::array_like::{array_like_length, array_like_receiver, array_like_values};
use super::indexing::{array_at_index, array_slice_end, array_slice_start};
use super::splice::{splice_delete_count, splice_start};

const MAX_ARRAY_LENGTH: usize = u32::MAX as usize;

pub(crate) fn native_array_prototype_concat(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let receiver = array_like_receiver(this_value, env);
    validate_array_species_constructor(receiver.clone(), "concat", env)?;

    let mut result = Vec::new();
    let mut holes = Vec::new();
    concat_array_item(&mut result, &mut holes, receiver, env)?;
    for value in argument_values.iter().cloned() {
        concat_array_item(&mut result, &mut holes, value, env)?;
    }
    Ok(Value::Array(ArrayRef::new_sparse(result, holes)))
}

fn validate_array_species_constructor(
    receiver: Value,
    method: &str,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    if !matches!(receiver, Value::Array(_)) {
        return Ok(());
    }
    match property_value(receiver, "constructor", env)? {
        Value::Undefined | Value::Function(_) | Value::Object(_) => Ok(()),
        _ => Err(RuntimeError {
            thrown: None,
            message: format!(
                "TypeError: Array.prototype.{method} constructor is not a constructor"
            ),
        }),
    }
}

pub(crate) fn native_array_prototype_slice(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let array_like = array_like_length(this_value, "Array.prototype.slice", env)?;
    let length = array_like.length;
    let start = array_slice_start(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
    )?;
    let end = array_slice_end(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    validate_array_species_constructor(array_like.receiver.clone(), "slice", env)?;

    let count = end.saturating_sub(start);
    if count > MAX_ARRAY_LENGTH {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid array length".to_owned(),
        });
    }
    if count == 0 {
        return Ok(Value::Array(ArrayRef::new(Vec::new())));
    }
    let mut result = Vec::with_capacity(count);
    let mut holes = Vec::new();
    for index in start..end {
        if has_property(array_like.receiver.clone(), env, &index.to_string())? {
            result.push(property_value(
                array_like.receiver.clone(),
                &index.to_string(),
                env,
            )?);
        } else {
            holes.push(result.len());
            result.push(Value::Undefined);
        }
    }
    Ok(Value::Array(ArrayRef::new_sparse(result, holes)))
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

fn concat_array_item(
    result: &mut Vec<Value>,
    holes: &mut Vec<usize>,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    if is_concat_spreadable(value.clone(), env)? {
        return concat_spread_array(result, holes, value, env);
    }
    result.push(value);
    Ok(())
}

fn is_concat_spreadable(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    if !is_object_like(&value) {
        return Ok(false);
    }
    if let Some(symbol) = symbol::is_concat_spreadable_symbol(env) {
        let spreadable = property_value_key(value.clone(), &PropertyKey::Symbol(symbol), env)?;
        if !matches!(spreadable, Value::Undefined) {
            return Ok(is_truthy(&spreadable));
        }
    }
    Ok(matches!(value, Value::Array(_)))
}

fn is_object_like(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_) | Value::Function(_) | Value::Array(_) | Value::Map(_) | Value::Set(_)
    )
}

fn concat_spread_array(
    result: &mut Vec<Value>,
    holes: &mut Vec<usize>,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let length = to_length_with_env(property_value(value.clone(), "length", env)?, env)?;
    for index in 0..length {
        let key = index.to_string();
        let target_index = result.len();
        if has_property(value.clone(), env, &key)? {
            result.push(property_value(value.clone(), &key, env)?);
        } else {
            result.push(Value::Undefined);
            holes.push(target_index);
        }
    }
    Ok(())
}
