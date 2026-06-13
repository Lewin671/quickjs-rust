use crate::{
    ArrayRef, PropertyKey, RuntimeError, Value, has_property, is_truthy, property_value,
    property_value_key, symbol, to_length_with_env, typed_array,
};

use super::indexing::{array_at_index, array_slice_end, array_slice_start};
use super::splice::{splice_start_with_env, to_spliced_delete_count};
use super::{
    array_like::array_like_length,
    species::{
        array_species_create, create_data_property_or_throw, set_array_like_length,
        validate_array_species_constructor,
    },
};
use crate::CallEnv;

const MAX_ARRAY_LENGTH: usize = u32::MAX as usize;
const MAX_SAFE_LENGTH: usize = (1usize << 53) - 1;

pub(crate) fn native_array_prototype_concat(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let receiver = array_like_length(this_value, "Array.prototype.concat", env)?.receiver;
    let result = array_species_create(receiver.clone(), 0, "concat", env)?;

    let mut next_index = 0;
    next_index = concat_array_item(result.clone(), next_index, receiver, env)?;
    for value in argument_values.iter().cloned() {
        next_index = concat_array_item(result.clone(), next_index, value, env)?;
    }
    set_array_like_length(result.clone(), next_index, env)?;
    Ok(result)
}

pub(crate) fn native_array_prototype_slice(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let array_like = array_like_length(this_value, "Array.prototype.slice", env)?;
    let length = array_like.length;
    let start = array_slice_start(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let end = array_slice_end(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let count = end.saturating_sub(start);
    if count > MAX_ARRAY_LENGTH {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid array length".to_owned(),
        });
    }
    validate_array_species_constructor(array_like.receiver.clone(), "slice", env)?;

    let result = array_species_create(array_like.receiver.clone(), count, "slice", env)?;
    for (target_index, source_index) in (start..end).enumerate() {
        let source_key = source_index.to_string();
        if has_property(array_like.receiver.clone(), env, &source_key)? {
            let value = property_value(array_like.receiver.clone(), &source_key, env)?;
            create_data_property_or_throw(result.clone(), target_index.to_string(), value, env)?;
        }
    }
    Ok(result)
}

pub(crate) fn native_array_prototype_to_reversed(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.toReversed", env)?;
    if source.length > MAX_ARRAY_LENGTH {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid array length".to_owned(),
        });
    }
    let mut values = Vec::with_capacity(source.length);
    for offset in 0..source.length {
        let from = source.length - offset - 1;
        values.push(property_value(
            source.receiver.clone(),
            &from.to_string(),
            env,
        )?);
    }
    Ok(Value::Array(ArrayRef::new(values)))
}

pub(crate) fn native_array_prototype_to_spliced(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.toSpliced", env)?;
    let length = source.length;
    let start = splice_start_with_env(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let delete_count = to_spliced_delete_count(length, start, argument_values, env)?;
    let items = if argument_values.len() > 2 {
        &argument_values[2..]
    } else {
        &[]
    };

    let new_length = length
        .saturating_sub(delete_count)
        .checked_add(items.len())
        .ok_or_else(|| RuntimeError {
            thrown: None,
            message: "TypeError: invalid array length".to_owned(),
        })?;
    if new_length > MAX_SAFE_LENGTH {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: invalid array length".to_owned(),
        });
    }
    if new_length > MAX_ARRAY_LENGTH {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid array length".to_owned(),
        });
    }

    let mut result = Vec::with_capacity(new_length);
    for index in 0..start {
        result.push(property_value(
            source.receiver.clone(),
            &index.to_string(),
            env,
        )?);
    }
    result.extend_from_slice(items);
    for index in start + delete_count..length {
        result.push(property_value(
            source.receiver.clone(),
            &index.to_string(),
            env,
        )?);
    }
    Ok(Value::Array(ArrayRef::new(result)))
}

pub(crate) fn native_array_prototype_with(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.with", env)?;
    if source.length > MAX_ARRAY_LENGTH {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid array length".to_owned(),
        });
    }
    let index = array_at_index(
        source.length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?
    .ok_or_else(|| RuntimeError {
        thrown: None,
        message: "RangeError: Array.prototype.with index out of range".to_owned(),
    })?;
    let replacement = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let mut values = Vec::with_capacity(source.length);
    for offset in 0..source.length {
        if offset == index {
            values.push(replacement.clone());
        } else {
            values.push(property_value(
                source.receiver.clone(),
                &offset.to_string(),
                env,
            )?);
        }
    }
    Ok(Value::Array(ArrayRef::new(values)))
}

fn concat_array_item(
    result: Value,
    next_index: usize,
    value: Value,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    if is_concat_spreadable(value.clone(), env)? {
        return concat_spread_array(result, next_index, value, env);
    }
    create_data_property_or_throw(result, next_index.to_string(), value, env)?;
    Ok(next_index + 1)
}

fn is_concat_spreadable(value: Value, env: &mut CallEnv) -> Result<bool, RuntimeError> {
    if !is_object_like(&value) {
        return Ok(false);
    }
    if let Some(symbol) = symbol::is_concat_spreadable_symbol(env) {
        let spreadable = property_value_key(value.clone(), &PropertyKey::Symbol(symbol), env)?;
        if !matches!(spreadable, Value::Undefined) {
            return Ok(is_truthy(&spreadable));
        }
    }
    if matches!(value, Value::Array(_)) {
        return Ok(true);
    }
    match value {
        Value::Proxy(proxy) => crate::proxy::proxy_target_is_array_result(&proxy),
        _ => Ok(false),
    }
}

fn is_object_like(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_)
            | Value::Function(_)
            | Value::Array(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Proxy(_)
    )
}

fn concat_spread_array(
    result: Value,
    next_index: usize,
    value: Value,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    let length = to_length_with_env(property_value(value.clone(), "length", env)?, env)?;
    let new_length = next_index
        .checked_add(length)
        .ok_or_else(concat_length_error)?;
    if new_length > MAX_SAFE_LENGTH {
        return Err(concat_length_error());
    }
    if let Some(next_index) =
        concat_spread_typed_array_fast_path(result.clone(), next_index, &value, length)?
    {
        return Ok(next_index);
    }
    for index in 0..length {
        let key = index.to_string();
        if has_property(value.clone(), env, &key)? {
            create_data_property_or_throw(
                result.clone(),
                (next_index + index).to_string(),
                property_value(value.clone(), &key, env)?,
                env,
            )?;
        }
    }
    Ok(new_length)
}

fn concat_spread_typed_array_fast_path(
    result: Value,
    next_index: usize,
    value: &Value,
    length: usize,
) -> Result<Option<usize>, RuntimeError> {
    let Value::Array(array) = &result else {
        return Ok(None);
    };
    let Value::Object(object) = value else {
        return Ok(None);
    };
    if !typed_array::is_typed_array_object(object) {
        return Ok(None);
    }

    let actual_length = typed_array::typed_array_length(object);
    let present_length = length.min(actual_length);
    for offset in 0..present_length {
        if !array.dense_data_property_eligible(next_index + offset) {
            return Ok(None);
        }
    }
    for (offset, value) in typed_array::read_view_elements(object, 0, present_length)
        .into_iter()
        .enumerate()
    {
        array.set(next_index + offset, value);
    }
    Ok(Some(next_index + length))
}

fn concat_length_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: invalid array length".to_owned(),
    }
}
