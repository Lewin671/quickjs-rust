use crate::{RuntimeError, Value, has_property, property_value, to_number_with_env};

use super::{
    array_like::array_like_length,
    mutation::{delete_array_like_property, set_array_like_property},
    species::{array_species_create, create_data_property_or_throw},
};
use crate::CallEnv;

const MAX_SAFE_INTEGER_LENGTH: usize = 9_007_199_254_740_991;
pub(crate) fn native_array_prototype_splice(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if matches!(this_value, Value::String(_)) {
        return Err(splice_length_error());
    }
    if let Value::Array(array) = &this_value
        && let Some(removed) = splice_plain_dense(array, argument_values, env)
    {
        return Ok(removed);
    }

    let source = array_like_length(this_value, "Array.prototype.splice", env)?;
    let receiver = source.receiver;
    let length = source.length;
    let start = splice_start_with_env(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let delete_count = splice_delete_count(length, start, argument_values, env)?;
    let items = if argument_values.len() > 2 {
        &argument_values[2..]
    } else {
        &[]
    };
    let new_length = length
        .checked_sub(delete_count)
        .and_then(|length| length.checked_add(items.len()))
        .filter(|length| *length <= MAX_SAFE_INTEGER_LENGTH)
        .ok_or_else(splice_length_error)?;
    let removed = splice_removed_elements(receiver.clone(), start, delete_count, env)?;
    move_splice_tail(
        receiver.clone(),
        length,
        start,
        delete_count,
        items.len(),
        env,
    )?;
    for (offset, item) in items.iter().cloned().enumerate() {
        set_array_like_property(receiver.clone(), (start + offset).to_string(), item, env)?;
    }
    set_array_like_property(
        receiver,
        "length".to_owned(),
        Value::Number(new_length as f64),
        env,
    )?;
    Ok(removed)
}

/// `splice` as one `Vec::splice` on an array nothing can observe being
/// edited in place (`with_plain_dense_mutation`), whose species is the
/// intrinsic Array and whose position arguments are already numbers, so no
/// coercion runs user code between reading the length and moving elements.
/// The specified algorithm formats every moved index into a string key:
/// SJCL's `b.splice(0, 16)` per hashed block made stanford-crypto-sha256
/// spend a third of its time there.
fn splice_plain_dense(
    array: &crate::ArrayRef,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Option<Value> {
    if !argument_values
        .iter()
        .take(2)
        .all(|value| matches!(value, Value::Number(_) | Value::Undefined))
        || !super::species::species_is_intrinsic_array(array, env)
    {
        return None;
    }
    let length = array.len();
    let start = splice_start_with_env(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )
    .ok()?;
    let delete_count = splice_delete_count(length, start, argument_values, env).ok()?;
    let items = argument_values.get(2..).unwrap_or(&[]);
    let removed = array.with_plain_dense_mutation(
        env,
        items.len().saturating_sub(delete_count),
        |elements| {
            (elements.len() == length).then(|| {
                elements
                    .splice(start..start + delete_count, items.iter().cloned())
                    .collect::<Vec<_>>()
            })
        },
    )??;
    Some(Value::Array(crate::ArrayRef::new(removed)))
}

pub(super) fn splice_start_with_env(
    length: usize,
    start: Value,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    let number = match start {
        Value::Undefined => 0.0,
        value => to_number_with_env(value, env)?,
    };
    if number.is_nan() {
        return Ok(0);
    }
    if number.is_infinite() {
        return Ok(if number.is_sign_negative() { 0 } else { length });
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
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    if argument_values.is_empty() {
        return Ok(0);
    }
    if argument_values.len() < 2 {
        return Ok(length.saturating_sub(start));
    }

    let number = to_number_with_env(argument_values[1].clone(), env)?;
    if number.is_nan() || number <= 0.0 {
        return Ok(0);
    }
    if number.is_infinite() {
        return Ok(length.saturating_sub(start));
    }
    Ok((number.trunc() as usize).min(length.saturating_sub(start)))
}

fn splice_removed_elements(
    receiver: Value,
    start: usize,
    delete_count: usize,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let removed = array_species_create(receiver.clone(), delete_count, "splice", env)?;
    for offset in 0..delete_count {
        let key = (start + offset).to_string();
        if has_property(receiver.clone(), env, &key)? {
            create_data_property_or_throw(
                removed.clone(),
                offset.to_string(),
                property_value(receiver.clone(), &key, env)?,
                env,
            )?;
        }
    }
    set_array_like_property(
        removed.clone(),
        "length".to_owned(),
        Value::Number(delete_count as f64),
        env,
    )?;
    Ok(removed)
}

fn move_splice_tail(
    receiver: Value,
    length: usize,
    start: usize,
    delete_count: usize,
    item_count: usize,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    match item_count.cmp(&delete_count) {
        std::cmp::Ordering::Less => {
            for index in start..(length - delete_count) {
                let from = (index + delete_count).to_string();
                let to = (index + item_count).to_string();
                if has_property(receiver.clone(), env, &from)? {
                    let value = property_value(receiver.clone(), &from, env)?;
                    set_array_like_property(receiver.clone(), to, value, env)?;
                } else {
                    delete_array_like_property(receiver.clone(), &to, env)?;
                }
            }
            for index in (length - delete_count + item_count)..length {
                delete_array_like_property(receiver.clone(), &index.to_string(), env)?;
            }
        }
        std::cmp::Ordering::Greater => {
            for index in (start..(length - delete_count)).rev() {
                let from = (index + delete_count).to_string();
                let to = (index + item_count).to_string();
                if has_property(receiver.clone(), env, &from)? {
                    let value = property_value(receiver.clone(), &from, env)?;
                    set_array_like_property(receiver.clone(), to, value, env)?;
                } else {
                    delete_array_like_property(receiver.clone(), &to, env)?;
                }
            }
        }
        std::cmp::Ordering::Equal => {}
    }
    Ok(())
}

fn splice_length_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.splice cannot set length".to_owned(),
    }
}

pub(super) fn to_spliced_delete_count(
    length: usize,
    start: usize,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    if argument_values.is_empty() {
        return Ok(0);
    }
    if argument_values.len() < 2 {
        return Ok(length.saturating_sub(start));
    }

    let number = to_number_with_env(argument_values[1].clone(), env)?;
    if number.is_nan() || number <= 0.0 {
        return Ok(0);
    }
    if number.is_infinite() {
        return Ok(length.saturating_sub(start));
    }
    Ok((number.trunc() as usize).min(length.saturating_sub(start)))
}
