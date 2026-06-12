use crate::{ArrayRef, RuntimeError, Value, has_property, property_value, to_number_with_env};

use super::{
    array_like::array_like_length,
    mutation::{delete_array_like_property, set_array_like_property},
    species::validate_array_species_constructor,
};
use crate::CallEnv;

const MAX_SAFE_INTEGER_LENGTH: usize = 9_007_199_254_740_991;
const MAX_ARRAY_LENGTH: usize = u32::MAX as usize;

pub(crate) fn native_array_prototype_splice(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if matches!(this_value, Value::String(_)) {
        return Err(splice_length_error());
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
    validate_array_species_constructor(receiver.clone(), "splice", env)?;
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
    if delete_count > MAX_ARRAY_LENGTH {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid array length".to_owned(),
        });
    }

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
    Ok(Value::Array(ArrayRef::new(removed)))
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
) -> Result<Vec<Value>, RuntimeError> {
    let mut removed = Vec::with_capacity(delete_count);
    for offset in 0..delete_count {
        let key = (start + offset).to_string();
        if has_property(receiver.clone(), env, &key)? {
            removed.push(property_value(receiver.clone(), &key, env)?);
        } else {
            removed.push(Value::Undefined);
        }
    }
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
