//! `%TypedArray.prototype%` write and ordering methods (ES2023 23.2.3):
//! `set`, `fill`, `copyWithin`, `reverse`, `sort`, `toReversed`, `toSorted`,
//! `with`.
//!
//! Writes route per-type conversion through [`element::set_view_elements`],
//! which persists the backing buffer in one pass and refreshes the materialized
//! index properties so ordinary `array[i]` reads stay consistent (indexed
//! *writes* through `array[i] = v` are still not hooked — see the campaign
//! notes).

use std::cmp::Ordering;
use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, array, array_buffer,
    call_function, object_prototype, property_value, to_number_with_env,
};

use super::element::{get_view_element, read_view_elements, set_view_elements};
use super::{
    MAX_TYPED_ARRAY_LENGTH, coerce_element, construct, is_big_int_kind, is_typed_array_object,
    typed_array_is_out_of_bounds, typed_array_kind, typed_array_length, validate_typed_array,
    validate_typed_array_write,
};
use crate::CallEnv;

// --- set --------------------------------------------------------------------

pub(crate) fn native_typed_array_prototype_set(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, _) = validate_typed_array_write(&this_value)?;
    let native = typed_array_kind(&object);
    let source = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let offset = set_offset(argument_values.get(1).cloned(), env)?;
    let (object, length) = validate_typed_array(&this_value)?;

    match source {
        Value::Object(ref source_object) if is_typed_array_object(source_object) => {
            set_from_typed_array(&object, native, length, source_object, offset, env)
        }
        other => set_from_array_like(&object, native, length, other, offset, env),
    }?;
    Ok(Value::Undefined)
}

fn set_offset(value: Option<Value>, env: &mut CallEnv) -> Result<usize, RuntimeError> {
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
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    if super::typed_array_buffer_detached(source) {
        return Err(array_buffer::detached_error());
    }
    if typed_array_is_out_of_bounds(source) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: TypedArray is out of bounds".to_owned(),
        });
    }
    let source_native = typed_array_kind(source);
    if is_big_int_kind(native) != is_big_int_kind(source_native) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: cannot mix BigInt and Number typed arrays".to_owned(),
        });
    }
    let source_length = typed_array_length(source);
    if offset
        .checked_add(source_length)
        .is_none_or(|end| end > length)
    {
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
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let source = array::array_like_length(source, "TypedArray.prototype.set", env)?;
    if offset
        .checked_add(source.length)
        .is_none_or(|end| end > length)
    {
        return Err(range_error("source is too large"));
    }
    for index in 0..source.length {
        let value = property_value(source.receiver.clone(), &index.to_string(), env)?;
        let coerced = coerce_element(native, value, env)?;
        set_view_elements(object, offset + index, [coerced]);
    }
    Ok(())
}

// --- fill -------------------------------------------------------------------

pub(crate) fn native_typed_array_prototype_fill(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array_write(&this_value)?;
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
    validate_typed_array(&this_value)?;
    if start < end {
        set_view_elements(&object, start, std::iter::repeat_n(value, end - start));
    }
    Ok(this_value)
}

// --- copyWithin -------------------------------------------------------------

pub(crate) fn native_typed_array_prototype_copy_within(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, initial_length) = validate_typed_array_write(&this_value)?;
    let target = relative_integer(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        0.0,
        env,
    )?;
    let start = relative_integer(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        0.0,
        env,
    )?;
    let end = match argument_values.get(2).cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => None,
        value => Some(relative_integer(value, initial_length as f64, env)?),
    };
    let target = relative_index_from_integer(target, initial_length);
    let start = relative_index_from_integer(start, initial_length);
    let end = relative_index_from_integer(end.unwrap_or(initial_length as f64), initial_length);
    let (_object, current_length) = validate_typed_array(&this_value)?;
    let count = end
        .saturating_sub(start)
        .min(initial_length.saturating_sub(target))
        .min(current_length.saturating_sub(target.max(start)));
    // Snapshot the source range to handle overlap correctly.
    let snapshot = read_view_elements(&object, start, count);
    set_view_elements(&object, target, snapshot);
    Ok(this_value)
}

// --- reverse / toReversed ---------------------------------------------------

pub(crate) fn native_typed_array_prototype_reverse(
    this_value: Value,
    _argument_values: &[Value],
    _env: &mut CallEnv,
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
    env: &mut CallEnv,
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
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array_write(&this_value)?;
    let comparator = sort_comparator(argument_values, "sort")?;
    let mut values: Vec<Value> = read_view_elements(&object, 0, length);
    sort_values(&mut values, comparator.as_ref(), env)?;
    set_view_elements(&object, 0, values);
    Ok(this_value)
}

pub(crate) fn native_typed_array_prototype_to_sorted(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
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
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    if values.len() < 2 {
        return Ok(());
    }
    let sorted = merge_sort_values(values.to_vec(), comparator, env)?;
    values.clone_from_slice(&sorted);
    Ok(())
}

fn merge_sort_values(
    mut values: Vec<Value>,
    comparator: Option<&Function>,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    if values.len() < 2 {
        return Ok(values);
    }
    let right = values.split_off(values.len() / 2);
    let left = merge_sort_values(values, comparator, env)?;
    let right = merge_sort_values(right, comparator, env)?;
    merge_sorted_values(left, right, comparator, env)
}

fn merge_sorted_values(
    left: Vec<Value>,
    right: Vec<Value>,
    comparator: Option<&Function>,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    let mut merged = Vec::with_capacity(left.len() + right.len());
    let mut left = left.into_iter().peekable();
    let mut right = right.into_iter().peekable();

    while left.peek().is_some() && right.peek().is_some() {
        let order = compare(left.peek().unwrap(), right.peek().unwrap(), comparator, env)?;
        if order == Ordering::Greater {
            merged.push(right.next().unwrap());
        } else {
            merged.push(left.next().unwrap());
        }
    }
    merged.extend(left);
    merged.extend(right);
    Ok(merged)
}

fn compare(
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
    env: &mut CallEnv,
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
    // Coerce the replacement value up front so type errors surface before the
    // current-index validation (BigInt arrays reject Number values, and vice
    // versa).
    let replacement = coerce_element(
        native,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if actual < 0.0 {
        return Err(range_error("invalid index"));
    }
    let actual = actual as usize;
    if actual >= typed_array_length(&object) || typed_array_is_out_of_bounds(&object) {
        return Err(range_error("invalid index"));
    }
    let mut values = Vec::with_capacity(length);
    for index in 0..length {
        let value = if index == actual {
            replacement.clone()
        } else {
            coerce_element(native, get_view_element(&object, index), env)?
        };
        values.push(value);
    }
    Ok(Value::Object(super::create_typed_array_of_kind(
        native, values, env,
    )))
}

// --- Uint8Array hex codecs ---------------------------------------------------

pub(crate) fn native_uint8_array_from_hex(
    argument_values: &[Value],
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    let source = hex_source(argument_values.first(), "Uint8Array.fromHex")?;
    let bytes = decode_hex_string(&source, MAX_TYPED_ARRAY_LENGTH)?;
    let values = bytes.into_iter().map(number_byte).collect();
    Ok(Value::Object(construct::create_with_values(
        NativeFunction::Uint8Array,
        values,
        env,
    )))
}

pub(crate) fn native_uint8_array_prototype_to_hex(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array(&this_value)?;
    if typed_array_kind(&object) != NativeFunction::Uint8Array {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Uint8Array.prototype.toHex requires a Uint8Array receiver"
                .to_owned(),
        });
    }
    let bytes: Vec<u8> = read_view_elements(&object, 0, length)
        .into_iter()
        .map(|value| match value {
            Value::Number(number) => number as u8,
            _ => 0,
        })
        .collect();
    Ok(Value::String(encode_hex_string(&bytes).into()))
}

pub(crate) fn native_uint8_array_prototype_set_from_hex(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (object, length) = validate_typed_array_write(&this_value)?;
    if typed_array_kind(&object) != NativeFunction::Uint8Array {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Uint8Array.prototype.setFromHex requires a Uint8Array receiver"
                .to_owned(),
        });
    }
    let source = hex_source(argument_values.first(), "Uint8Array.prototype.setFromHex")?;

    let chars: Vec<char> = source.chars().collect();
    if chars.len() % 2 != 0 {
        return Err(syntax_error(
            "hex string must contain an even number of digits",
        ));
    }

    let mut values = Vec::new();
    let mut read = 0usize;
    while values.len() < length && read < chars.len() {
        let high = match hex_value(chars[read]) {
            Some(value) => value,
            None => {
                set_view_elements(&object, 0, values.into_iter().map(number_byte));
                return Err(syntax_error("invalid hex digit"));
            }
        };
        let low = match hex_value(chars[read + 1]) {
            Some(value) => value,
            None => {
                set_view_elements(&object, 0, values.into_iter().map(number_byte));
                return Err(syntax_error("invalid hex digit"));
            }
        };
        values.push((high << 4) | low);
        read += 2;
    }

    let written = values.len();
    set_view_elements(&object, 0, values.into_iter().map(number_byte));
    Ok(set_from_hex_result(read, written, env))
}

fn hex_source(value: Option<&Value>, method: &str) -> Result<String, RuntimeError> {
    match value {
        Some(Value::String(source)) => Ok(source.clone().to_string()),
        _ => Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: {method} requires a string"),
        }),
    }
}

fn decode_hex_string(source: &str, max_length: usize) -> Result<Vec<u8>, RuntimeError> {
    let chars: Vec<char> = source.chars().collect();
    if chars.len() % 2 != 0 {
        return Err(syntax_error(
            "hex string must contain an even number of digits",
        ));
    }
    let byte_length = chars.len() / 2;
    if byte_length > max_length {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid typed array length".to_owned(),
        });
    }
    let mut values = Vec::with_capacity(byte_length);
    let mut read = 0usize;
    while read < chars.len() {
        let high = hex_value(chars[read]).ok_or_else(|| syntax_error("invalid hex digit"))?;
        let low = hex_value(chars[read + 1]).ok_or_else(|| syntax_error("invalid hex digit"))?;
        values.push((high << 4) | low);
        read += 2;
    }
    Ok(values)
}

fn encode_hex_string(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}

fn hex_value(ch: char) -> Option<u8> {
    match ch {
        '0'..='9' => Some(ch as u8 - b'0'),
        'a'..='f' => Some(ch as u8 - b'a' + 10),
        'A'..='F' => Some(ch as u8 - b'A' + 10),
        _ => None,
    }
}

fn number_byte(value: u8) -> Value {
    Value::Number(value as f64)
}

fn set_from_hex_result(read: usize, written: usize, env: &CallEnv) -> Value {
    let result = ObjectRef::with_prototype(HashMap::new(), object_prototype(env));
    result.define_property(
        "read".to_owned(),
        Property::enumerable(Value::Number(read as f64)),
    );
    result.define_property(
        "written".to_owned(),
        Property::enumerable(Value::Number(written as f64)),
    );
    Value::Object(result)
}

fn syntax_error(message: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("SyntaxError: {message}"),
    }
}

// --- shared helpers ---------------------------------------------------------

fn relative_index(
    value: Value,
    length: usize,
    default: i64,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    let relative = relative_integer(value, default as f64, env)?;
    Ok(relative_index_from_integer(relative, length))
}

fn relative_integer(value: Value, default: f64, env: &mut CallEnv) -> Result<f64, RuntimeError> {
    Ok(match value {
        Value::Undefined => default,
        other => {
            let number = to_number_with_env(other, env)?;
            if number.is_nan() { 0.0 } else { number.trunc() }
        }
    })
}

fn relative_index_from_integer(relative: f64, length: usize) -> usize {
    let resolved = if relative < 0.0 {
        (length as f64 + relative).max(0.0)
    } else {
        relative.min(length as f64)
    };
    resolved as usize
}

fn range_error(message: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("RangeError: {message}"),
    }
}
