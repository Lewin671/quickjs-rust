use std::cmp::Ordering;

use crate::{
    ArrayRef, Function, RuntimeError, Value, call_function, has_property, property_value,
    to_js_string_with_env, to_number_with_env,
};

use super::{
    array_like::array_like_length,
    mutation::{delete_array_like_property, set_array_like_property},
};
use crate::CallEnv;

const MAX_ARRAY_LENGTH: usize = u32::MAX as usize;

pub(crate) fn native_array_prototype_sort(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let comparator = array_sort_comparator(argument_values, "Array.prototype.sort")?;
    let source = array_like_length(this_value, "Array.prototype.sort", env)?;
    let sorted = sorted_present_array_like_values(
        source.receiver.clone(),
        source.length,
        comparator.as_ref(),
        env,
    )?;
    write_sorted_array_like_values(source.receiver.clone(), source.length, sorted, env)?;
    Ok(source.receiver)
}

pub(crate) fn native_array_prototype_to_sorted(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let comparator = array_sort_comparator(argument_values, "Array.prototype.toSorted")?;
    let values = to_sorted_array_like_values(this_value, env)?;
    Ok(Value::Array(ArrayRef::new(sorted_array_values(
        values,
        comparator.as_ref(),
        env,
    )?)))
}

fn to_sorted_array_like_values(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.toSorted", env)?;
    if source.length > MAX_ARRAY_LENGTH {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid array length".to_owned(),
        });
    }
    let mut values = Vec::with_capacity(source.length);
    for index in 0..source.length {
        values.push(property_value(
            source.receiver.clone(),
            &index.to_string(),
            env,
        )?);
    }
    Ok(values)
}

fn sorted_array_values(
    values: Vec<Value>,
    comparator: Option<&Function>,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    let mut defined = Vec::new();
    let mut undefined_count = 0;
    for value in values {
        if matches!(value, Value::Undefined) {
            undefined_count += 1;
        } else {
            defined.push(value);
        }
    }

    insertion_sort(&mut defined, comparator, env)?;
    defined.extend(std::iter::repeat_n(Value::Undefined, undefined_count));
    Ok(defined)
}

fn sorted_present_array_like_values(
    receiver: Value,
    length: usize,
    comparator: Option<&Function>,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    let mut values = Vec::new();
    for index in 0..length {
        let key = index.to_string();
        if has_property(receiver.clone(), env, &key)? {
            values.push(property_value(receiver.clone(), &key, env)?);
        }
    }
    sorted_array_values(values, comparator, env)
}

fn write_sorted_array_like_values(
    receiver: Value,
    length: usize,
    values: Vec<Value>,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let item_count = values.len();
    for (index, value) in values.into_iter().enumerate() {
        set_array_like_property(receiver.clone(), index.to_string(), value, env)?;
    }
    for index in item_count..length {
        delete_array_like_property(receiver.clone(), &index.to_string(), env)?;
    }
    Ok(())
}

fn array_sort_comparator(
    argument_values: &[Value],
    context: &str,
) -> Result<Option<Function>, RuntimeError> {
    match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => Ok(None),
        Value::Function(function) => Ok(Some(function)),
        _ => Err(RuntimeError {
            thrown: None,
            message: format!("{context} comparator must be callable"),
        }),
    }
}

fn insertion_sort(
    values: &mut [Value],
    comparator: Option<&Function>,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    for index in 1..values.len() {
        let mut candidate = index;
        while candidate > 0
            && compare_values(&values[candidate], &values[candidate - 1], comparator, env)?
                == Ordering::Less
        {
            values.swap(candidate, candidate - 1);
            candidate -= 1;
        }
    }
    Ok(())
}

fn compare_values(
    left: &Value,
    right: &Value,
    comparator: Option<&Function>,
    env: &mut CallEnv,
) -> Result<Ordering, RuntimeError> {
    if let Some(function) = comparator {
        let result = call_function(
            Value::Function(function.clone()),
            Value::Undefined,
            vec![left.clone(), right.clone()],
            env,
            false,
        )?;
        let order = to_number_with_env(result, env)?;
        if order.is_nan() || order == 0.0 {
            Ok(Ordering::Equal)
        } else if order < 0.0 {
            Ok(Ordering::Less)
        } else {
            Ok(Ordering::Greater)
        }
    } else {
        Ok(to_js_string_with_env(left.clone(), env)?
            .cmp(&to_js_string_with_env(right.clone(), env)?))
    }
}
