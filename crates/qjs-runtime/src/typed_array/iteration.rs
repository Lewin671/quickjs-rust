//! `%TypedArray.prototype%` iteration and read-family methods (ES2023 23.2.3).
//!
//! Each method brand-checks its receiver and validates the backing buffer is
//! attached, then reads elements straight from the buffer (the source of truth)
//! so values stay correct even if index properties drift.

use crate::{
    ObjectRef, RuntimeError, Value, array_buffer, call_function, is_truthy, to_js_string_with_env,
    to_number_with_env,
};

use super::element::{ViewSnapshot, get_view_element, read_view_elements};
use super::{
    bytes_per_element, typed_array_buffer, typed_array_byte_offset, typed_array_kind,
    validate_typed_array, validate_typed_array_length,
};
use crate::CallEnv;

// --- shared iteration scaffolding -------------------------------------------

struct Iteration {
    receiver: Value,
    object: ObjectRef,
    callback: Value,
    callback_this: Value,
    length: usize,
}

fn prepare_iteration(
    method: &str,
    this_value: Value,
    argument_values: &[Value],
) -> Result<Iteration, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: %TypedArray%.prototype.{method} callback is not callable"),
        });
    }
    Ok(Iteration {
        receiver: this_value,
        object,
        callback,
        callback_this: argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
    })
}

fn call_callback(
    iteration: &Iteration,
    value: Value,
    index: usize,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    call_function(
        iteration.callback.clone(),
        iteration.callback_this.clone(),
        vec![
            value,
            Value::Number(index as f64),
            iteration.receiver.clone(),
        ],
        env,
        false,
    )
}

/// A relative index argument (`at`, `slice`, `subarray`, …) resolved against
/// `length`: negative values count from the end, clamped to `[0, length]`.
fn relative_index(
    value: Value,
    length: usize,
    default: i64,
    env: &mut CallEnv,
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

// --- at / indexOf / lastIndexOf / includes ----------------------------------

pub(crate) fn native_typed_array_prototype_at(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    let argument = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let number = to_number_with_env(argument, env)?;
    let relative = if number.is_nan() { 0.0 } else { number.trunc() };
    let index = if relative < 0.0 {
        length as f64 + relative
    } else {
        relative
    };
    if index < 0.0 || index >= length as f64 {
        return Ok(Value::Undefined);
    }
    Ok(get_view_element(&object, index as usize))
}

pub(crate) fn native_typed_array_prototype_index_of(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    if length == 0 {
        return Ok(Value::Number(-1.0));
    }
    let search = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let start = search_start_index(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
        env,
    )?;
    let scanned = read_view_elements(&object, start, length - start);
    for (offset, value) in scanned.iter().enumerate() {
        if strict_same(value, &search) {
            return Ok(Value::Number((start + offset) as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

pub(crate) fn native_typed_array_prototype_last_index_of(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    if length == 0 {
        return Ok(Value::Number(-1.0));
    }
    let search = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let start = match argument_values.get(1).cloned() {
        None | Some(Value::Undefined) => length - 1,
        Some(other) => {
            let number = to_number_with_env(other, env)?;
            let from = if number.is_nan() { 0.0 } else { number.trunc() };
            if from >= 0.0 {
                (from as usize).min(length - 1)
            } else {
                let candidate = length as f64 + from;
                if candidate < 0.0 {
                    return Ok(Value::Number(-1.0));
                }
                candidate as usize
            }
        }
    };
    let scanned = read_view_elements(&object, 0, start + 1);
    for index in (0..=start).rev() {
        if strict_same(&scanned[index], &search) {
            return Ok(Value::Number(index as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

pub(crate) fn native_typed_array_prototype_includes(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    if length == 0 {
        return Ok(Value::Boolean(false));
    }
    let search = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let start = search_start_index(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
        env,
    )?;
    let scanned = read_view_elements(&object, start, length - start);
    for value in &scanned {
        if same_value_zero(value, &search) {
            return Ok(Value::Boolean(true));
        }
    }
    Ok(Value::Boolean(false))
}

fn search_start_index(
    value: Value,
    length: usize,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    let number = to_number_with_env(value, env)?;
    let from = if number.is_nan() { 0.0 } else { number.trunc() };
    Ok(if from >= 0.0 {
        (from as usize).min(length)
    } else {
        (length as f64 + from).max(0.0) as usize
    })
}

fn strict_same(left: &Value, right: &Value) -> bool {
    left == right
}

fn same_value_zero(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => {
            left == right || (left.is_nan() && right.is_nan())
        }
        _ => left == right,
    }
}

// --- join / toString / toLocaleString ---------------------------------------

pub(crate) fn native_typed_array_prototype_join(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    let separator = match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => ",".to_owned(),
        value => to_js_string_with_env(value, env)?,
    };
    let elements = read_view_elements(&object, 0, length);
    let mut parts = Vec::with_capacity(length);
    for element in elements {
        parts.push(to_js_string_with_env(element, env)?);
    }
    Ok(Value::String(parts.join(&separator)))
}

pub(crate) fn native_typed_array_prototype_to_string(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    // %TypedArray.prototype.toString% is %Array.prototype.toString%, which calls
    // the receiver's own `join` and falls back to Object.prototype.toString.
    native_typed_array_prototype_join(this_value, argument_values, env)
}

pub(crate) fn native_typed_array_prototype_to_locale_string(
    this_value: Value,
    _argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    let elements = read_view_elements(&object, 0, length);
    let mut parts = Vec::with_capacity(length);
    for element in elements {
        let part = call_to_locale_string(element, env)?;
        parts.push(part);
    }
    Ok(Value::String(parts.join(",")))
}

fn call_to_locale_string(value: Value, env: &mut CallEnv) -> Result<String, RuntimeError> {
    // Per spec, invoke each element's `toLocaleString`. Numbers/BigInts have
    // their own; we route through the generic string conversion which already
    // mirrors `Number.prototype.toLocaleString` basics for these element types.
    to_js_string_with_env(value, env)
}

// --- keys / values / entries / Symbol.iterator ------------------------------

pub(crate) fn native_typed_array_prototype_keys(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, _) = validate_typed_array(&this_value)?;
    Ok(crate::array::array_key_iterator(Value::Object(object), env))
}

pub(crate) fn native_typed_array_prototype_values(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, _) = validate_typed_array(&this_value)?;
    Ok(crate::array::array_value_iterator(
        Value::Object(object),
        env,
    ))
}

pub(crate) fn native_typed_array_prototype_entries(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, _) = validate_typed_array(&this_value)?;
    Ok(crate::array::array_key_value_iterator(
        Value::Object(object),
        env,
    ))
}

// --- forEach / some / every / find* -----------------------------------------

pub(crate) fn native_typed_array_prototype_for_each(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_iteration("forEach", this_value, argument_values)?;
    let snapshot = ViewSnapshot::capture(&iteration.object);
    for index in 0..iteration.length {
        let value = snapshot.get(index);
        call_callback(&iteration, value, index, env)?;
    }
    Ok(Value::Undefined)
}

pub(crate) fn native_typed_array_prototype_some(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_iteration("some", this_value, argument_values)?;
    let snapshot = ViewSnapshot::capture(&iteration.object);
    for index in 0..iteration.length {
        let value = snapshot.get(index);
        if is_truthy(&call_callback(&iteration, value, index, env)?) {
            return Ok(Value::Boolean(true));
        }
    }
    Ok(Value::Boolean(false))
}

pub(crate) fn native_typed_array_prototype_every(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_iteration("every", this_value, argument_values)?;
    let snapshot = ViewSnapshot::capture(&iteration.object);
    for index in 0..iteration.length {
        let value = snapshot.get(index);
        if !is_truthy(&call_callback(&iteration, value, index, env)?) {
            return Ok(Value::Boolean(false));
        }
    }
    Ok(Value::Boolean(true))
}

pub(crate) fn native_typed_array_prototype_find(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_iteration("find", this_value, argument_values)?;
    let snapshot = ViewSnapshot::capture(&iteration.object);
    for index in 0..iteration.length {
        let value = snapshot.get(index);
        if is_truthy(&call_callback(&iteration, value.clone(), index, env)?) {
            return Ok(value);
        }
    }
    Ok(Value::Undefined)
}

pub(crate) fn native_typed_array_prototype_find_index(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_iteration("findIndex", this_value, argument_values)?;
    let snapshot = ViewSnapshot::capture(&iteration.object);
    for index in 0..iteration.length {
        let value = snapshot.get(index);
        if is_truthy(&call_callback(&iteration, value, index, env)?) {
            return Ok(Value::Number(index as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

pub(crate) fn native_typed_array_prototype_find_last(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_iteration("findLast", this_value, argument_values)?;
    let snapshot = ViewSnapshot::capture(&iteration.object);
    for index in (0..iteration.length).rev() {
        let value = snapshot.get(index);
        if is_truthy(&call_callback(&iteration, value.clone(), index, env)?) {
            return Ok(value);
        }
    }
    Ok(Value::Undefined)
}

pub(crate) fn native_typed_array_prototype_find_last_index(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_iteration("findLastIndex", this_value, argument_values)?;
    let snapshot = ViewSnapshot::capture(&iteration.object);
    for index in (0..iteration.length).rev() {
        let value = snapshot.get(index);
        if is_truthy(&call_callback(&iteration, value, index, env)?) {
            return Ok(Value::Number(index as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

// --- map / filter / reduce / reduceRight ------------------------------------

pub(crate) fn native_typed_array_prototype_map(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_iteration("map", this_value, argument_values)?;
    let native = typed_array_kind(&iteration.object);
    let snapshot = ViewSnapshot::capture(&iteration.object);
    let mut mapped = Vec::with_capacity(iteration.length);
    for index in 0..iteration.length {
        let value = snapshot.get(index);
        let result = call_callback(&iteration, value, index, env)?;
        mapped.push(super::coerce_element(native, result, env)?);
    }
    Ok(Value::Object(super::create_typed_array_of_kind(
        native, mapped, env,
    )))
}

pub(crate) fn native_typed_array_prototype_filter(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_iteration("filter", this_value, argument_values)?;
    let native = typed_array_kind(&iteration.object);
    let snapshot = ViewSnapshot::capture(&iteration.object);
    let mut kept = Vec::new();
    for index in 0..iteration.length {
        let value = snapshot.get(index);
        if is_truthy(&call_callback(&iteration, value.clone(), index, env)?) {
            kept.push(value);
        }
    }
    Ok(Value::Object(super::create_typed_array_of_kind(
        native, kept, env,
    )))
}

pub(crate) fn native_typed_array_prototype_reduce(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    let callback = require_callback("reduce", argument_values)?;
    let snapshot = ViewSnapshot::capture(&object);
    let (mut accumulator, mut index) = if argument_values.len() >= 2 {
        (argument_values[1].clone(), 0)
    } else {
        if length == 0 {
            return Err(reduce_empty_error());
        }
        (snapshot.get(0), 1)
    };
    while index < length {
        let value = snapshot.get(index);
        accumulator = call_function(
            callback.clone(),
            Value::Undefined,
            vec![
                accumulator,
                value,
                Value::Number(index as f64),
                this_value.clone(),
            ],
            env,
            false,
        )?;
        index += 1;
    }
    Ok(accumulator)
}

pub(crate) fn native_typed_array_prototype_reduce_right(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    let callback = require_callback("reduceRight", argument_values)?;
    let snapshot = ViewSnapshot::capture(&object);
    let (mut accumulator, mut next) = if argument_values.len() >= 2 {
        (argument_values[1].clone(), length)
    } else {
        if length == 0 {
            return Err(reduce_empty_error());
        }
        (snapshot.get(length - 1), length - 1)
    };
    while next > 0 {
        next -= 1;
        let value = snapshot.get(next);
        accumulator = call_function(
            callback.clone(),
            Value::Undefined,
            vec![
                accumulator,
                value,
                Value::Number(next as f64),
                this_value.clone(),
            ],
            env,
            false,
        )?;
    }
    Ok(accumulator)
}

fn require_callback(method: &str, argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(callback, Value::Function(_)) {
        Ok(callback)
    } else {
        Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: %TypedArray%.prototype.{method} callback is not callable"),
        })
    }
}

fn reduce_empty_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Reduce of empty array with no initial value".to_owned(),
    }
}

// --- slice / subarray --------------------------------------------------------

pub(crate) fn native_typed_array_prototype_slice(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    let native = typed_array_kind(&object);
    let start = relative_index(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        length,
        0,
        env,
    )?;
    let end = relative_index(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
        length as i64,
        env,
    )?;
    let values = if start < end {
        read_view_elements(&object, start, end - start)
    } else {
        Vec::new()
    };
    Ok(Value::Object(super::create_typed_array_of_kind(
        native, values, env,
    )))
}

fn validate_subarray_range(
    object: &ObjectRef,
    start: usize,
    count: usize,
) -> Result<(), RuntimeError> {
    let Some(buffer) = typed_array_buffer(object) else {
        return Ok(());
    };
    let buffer_byte_length = array_buffer::array_buffer_bytes(&buffer).len();
    let element = bytes_per_element(typed_array_kind(object));
    let byte_start = typed_array_byte_offset(object)
        .checked_add(
            start
                .checked_mul(element)
                .ok_or_else(invalid_subarray_range_error)?,
        )
        .ok_or_else(invalid_subarray_range_error)?;
    let byte_length = count
        .checked_mul(element)
        .ok_or_else(invalid_subarray_range_error)?;
    if byte_start > buffer_byte_length
        || byte_start
            .checked_add(byte_length)
            .is_none_or(|end| end > buffer_byte_length)
    {
        return Err(invalid_subarray_range_error());
    }
    Ok(())
}

fn invalid_subarray_range_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "RangeError: invalid typed array subarray range".to_owned(),
    }
}

pub(crate) fn native_typed_array_prototype_subarray(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    // subarray shares the backing buffer; without a shared-buffer view model we
    // produce an independent copy of the range, which keeps element values and
    // length correct (the data aliasing is a known simplification, noted in the
    // task file). A future shared-view slot would replace this copy.
    let (object, length) = validate_typed_array_length(&this_value)?;
    let native = typed_array_kind(&object);
    let start = relative_index(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        length,
        0,
        env,
    )?;
    let end = relative_index(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
        length as i64,
        env,
    )?;
    let values = if start < end {
        validate_subarray_range(&object, start, end - start)?;
        read_view_elements(&object, start, end - start)
    } else {
        validate_subarray_range(&object, start, 0)?;
        Vec::new()
    };
    Ok(Value::Object(super::create_typed_array_of_kind(
        native, values, env,
    )))
}
