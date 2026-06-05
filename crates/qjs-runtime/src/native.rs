use std::collections::HashMap;

mod arrays;
mod core;
mod date;
mod errors;
mod json;
mod maps;
mod math;
mod numbers;
mod objects;
mod promises;
mod reflect;
mod regexp;
mod sets;
mod strings;
mod weak_maps;
mod weak_sets;

use crate::{Function, NativeFunction, RuntimeError, Value};

type NativeCallResult = Result<Option<Value>, RuntimeError>;

pub(crate) fn call_native_function(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: Vec<Value>,
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if let Some(value) =
        arrays::call_array_native(native, this_value.clone(), &argument_values, env)?
    {
        return Ok(value);
    }

    if let Some(value) = date::call_date_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) = math::call_math_native(native, &argument_values)? {
        return Ok(value);
    }

    if let Some(value) = errors::call_error_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
    )? {
        return Ok(value);
    }

    if let Some(value) = json::call_json_native(native, &argument_values, env)? {
        return Ok(value);
    }

    if let Some(value) = maps::call_map_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) = sets::call_set_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) = weak_maps::call_weak_map_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) = weak_sets::call_weak_set_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) = numbers::call_number_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
    )? {
        return Ok(value);
    }

    if let Some(value) = promises::call_promise_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) = objects::call_object_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) = reflect::call_reflect_native(native, &argument_values, env)? {
        return Ok(value);
    }

    if let Some(value) = regexp::call_regexp_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) = strings::call_string_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    core::call_core_native(
        function,
        native,
        this_value,
        &argument_values,
        is_construct,
        env,
    )
}
