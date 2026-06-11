//! `%TypedArray.prototype%` write and ordering methods (ES2023 23.2.3):
//! `set`, `fill`, `copyWithin`, `reverse`, `sort`, `toReversed`, `toSorted`,
//! `with`.
//!
//! Writes route per-type conversion through [`element::set_view_elements`],
//! which persists the backing buffer in one pass and refreshes the materialized
//! index properties so ordinary `array[i]` reads stay consistent (indexed
//! *writes* through `array[i] = v` are still not hooked — see the campaign
//! notes).

use std::{cmp::Ordering, collections::HashMap};

use crate::{
    Function, NativeFunction, ObjectRef, RuntimeError, Value, array, array_buffer, call_function,
    to_number_with_env,
};

use super::element::{read_view_elements, set_view_elements};
use super::{
    coerce_element, is_big_int_kind, is_typed_array_object, typed_array_kind, typed_array_length,
    validate_typed_array,
};

// --- set --------------------------------------------------------------------

pub(crate) fn native_typed_array_prototype_set(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    let native = typed_array_kind(&object);
    let source = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let offset = set_offset(argument_values.get(1).cloned(), env)?;

    match source {
        Value::Object(ref source_object) if is_typed_array_object(source_object) => {
            set_from_typed_array(&object, native, length, source_object, offset, env)
        }
        other => set_from_array_like(&object, native, length, other, offset, env),
    }?;
    Ok(Value::Undefined)
}

fn set_offset(
    value: Option<Value>,
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    let number = to_number_with_env(value.unwrap_or(Value::Undefined), env)?;
    let integer = if number.is_nan() { 0.0 } else { number.trunc() };
    if integer < 0.0 {
        return Err(range_error("offset is out of bounds"));
    }
    Ok(integer as usize)
}

fn set_from_typed_array(
    object: &ObjectRef,
    native: NativeFunction,
    length: usize,
    source: &ObjectRef,
    offset: usize,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    if super::typed_array_buffer_detached(source) {
        return Err(array_buffer::detached_error());
    }
    let source_native = typed_array_kind(source);
    if is_big_int_kind(native) != is_big_int_kind(source_native) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: cannot mix BigInt and Number typed arrays".to_owned(),
        });
    }
    let source_length = typed_array_length(source);
    if offset + source_length > length {
        return Err(range_error("source is too large"));
    }
    // Snapshot the source first so overlapping buffers behave per spec.
    let values = read_view_elements(source, 0, source_length);
    let mut coerced = Vec::with_capacity(values.len());
    for value in values {
        coerced.push(coerce_element(native, value, env)?);
    }
    set_view_elements(object, offset, coerced);
    Ok(())
}

fn set_from_array_like(
    object: &ObjectRef,
    native: NativeFunction,
    length: usize,
    source: Value,
    offset: usize,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let values = array::array_like_values_with_env(source, "TypedArray.prototype.set", env)?;
    if offset + values.len() > length {
        return Err(range_error("source is too large"));
    }
    let mut coerced = Vec::with_capacity(values.len());
    for value in values {
        coerced.push(coerce_element(native, value, env)?);
    }
    set_view_elements(object, offset, coerced);
    Ok(())
}

// --- fill -------------------------------------------------------------------

pub(crate) fn native_typed_array_prototype_fill(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    let native = typed_array_kind(&object);
    let value = coerce_element(
        native,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let start = relative_index(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
        0,
        env,
    )?;
    let end = relative_index(
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
        length,
        length as i64,
        env,
    )?;
    if start < end {
        set_view_elements(&object, start, std::iter::repeat_n(value, end - start));
    }
    Ok(this_value)
}

// --- copyWithin -------------------------------------------------------------

pub(crate) fn native_typed_array_prototype_copy_within(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    let target = relative_index(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        length,
        0,
        env,
    )?;
    let start = relative_index(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
        0,
        env,
    )?;
    let end = relative_index(
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
        length,
        length as i64,
        env,
    )?;
    let count = end.saturating_sub(start).min(length.saturating_sub(target));
    // Snapshot the source range to handle overlap correctly.
    let snapshot = read_view_elements(&object, start, count);
    set_view_elements(&object, target, snapshot);
    Ok(this_value)
}

// --- reverse / toReversed ---------------------------------------------------

pub(crate) fn native_typed_array_prototype_reverse(
    this_value: Value,
    _argument_values: &[Value],
    _env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    let mut values: Vec<Value> = read_view_elements(&object, 0, length);
    values.reverse();
    set_view_elements(&object, 0, values);
    Ok(this_value)
}

pub(crate) fn native_typed_array_prototype_to_reversed(
    this_value: Value,
    _argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    let native = typed_array_kind(&object);
    let mut values: Vec<Value> = read_view_elements(&object, 0, length);
    values.reverse();
    Ok(Value::Object(super::create_typed_array_of_kind(
        native, values, env,
    )))
}

// --- sort / toSorted --------------------------------------------------------

pub(crate) fn native_typed_array_prototype_sort(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    let comparator = sort_comparator(argument_values, "sort")?;
    let mut values: Vec<Value> = read_view_elements(&object, 0, length);
    sort_values(&mut values, comparator.as_ref(), env)?;
    set_view_elements(&object, 0, values);
    Ok(this_value)
}

pub(crate) fn native_typed_array_prototype_to_sorted(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    let native = typed_array_kind(&object);
    let comparator = sort_comparator(argument_values, "toSorted")?;
    let mut values: Vec<Value> = read_view_elements(&object, 0, length);
    sort_values(&mut values, comparator.as_ref(), env)?;
    Ok(Value::Object(super::create_typed_array_of_kind(
        native, values, env,
    )))
}

fn sort_comparator(
    argument_values: &[Value],
    context: &str,
) -> Result<Option<Function>, RuntimeError> {
    match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => Ok(None),
        Value::Function(function) => Ok(Some(function)),
        _ => Err(RuntimeError {
            thrown: None,
            message: format!(
                "TypeError: %TypedArray%.prototype.{context} comparator must be callable"
            ),
        }),
    }
}

/// Stable sort by the TypedArray default numeric ordering, or by the result of
/// `comparator` when supplied.
fn sort_values(
    values: &mut [Value],
    comparator: Option<&Function>,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    // Insertion sort keeps stability and lets the comparator observe values
    // left-to-right (matching the array implementation in this codebase).
    for index in 1..values.len() {
        let mut candidate = index;
        while candidate > 0
            && compare(&values[candidate], &values[candidate - 1], comparator, env)?
                == Ordering::Less
        {
            values.swap(candidate, candidate - 1);
            candidate -= 1;
        }
    }
    Ok(())
}

fn compare(
    left: &Value,
    right: &Value,
    comparator: Option<&Function>,
    env: &mut HashMap<String, Value>,
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
        return Ok(if order.is_nan() || order == 0.0 {
            Ordering::Equal
        } else if order < 0.0 {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    Ok(default_numeric_order(left, right))
}

/// Default TypedArray numeric ordering: ascending, NaN sorts to the end, and
/// `-0` precedes `+0`. BigInt elements compare numerically.
fn default_numeric_order(left: &Value, right: &Value) -> Ordering {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => number_order(*a, *b),
        (Value::BigInt(a), Value::BigInt(b)) => a.cmp(b),
        _ => Ordering::Equal,
    }
}

fn number_order(a: f64, b: f64) -> Ordering {
    if a.is_nan() {
        return if b.is_nan() {
            Ordering::Equal
        } else {
            Ordering::Greater
        };
    }
    if b.is_nan() {
        return Ordering::Less;
    }
    if a < b {
        Ordering::Less
    } else if a > b {
        Ordering::Greater
    } else if a == 0.0 && b == 0.0 {
        // -0 before +0.
        a.is_sign_negative().cmp(&b.is_sign_negative()).reverse()
    } else {
        Ordering::Equal
    }
}

// --- with -------------------------------------------------------------------

pub(crate) fn native_typed_array_prototype_with(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    let native = typed_array_kind(&object);
    let relative = {
        let number = to_number_with_env(
            argument_values.first().cloned().unwrap_or(Value::Undefined),
            env,
        )?;
        if number.is_nan() { 0.0 } else { number.trunc() }
    };
    let actual = if relative < 0.0 {
        length as f64 + relative
    } else {
        relative
    };
    if actual < 0.0 || actual >= length as f64 {
        return Err(range_error("invalid index"));
    }
    let actual = actual as usize;
    // Coerce the replacement value up front so type errors surface before the
    // copy (BigInt arrays reject Number values, and vice versa).
    let replacement = coerce_element(
        native,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let mut values: Vec<Value> = read_view_elements(&object, 0, length);
    values[actual] = replacement;
    Ok(Value::Object(super::create_typed_array_of_kind(
        native, values, env,
    )))
}

// --- shared helpers ---------------------------------------------------------

fn relative_index(
    value: Value,
    length: usize,
    default: i64,
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    let relative = match value {
        Value::Undefined => default as f64,
        other => {
            let number = to_number_with_env(other, env)?;
            if number.is_nan() { 0.0 } else { number.trunc() }
        }
    };
    let resolved = if relative < 0.0 {
        (length as f64 + relative).max(0.0)
    } else {
        relative.min(length as f64)
    };
    Ok(resolved as usize)
}

fn range_error(message: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("RangeError: {message}"),
    }
}
