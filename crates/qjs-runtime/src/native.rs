mod array_buffers;
mod arrays;
mod atomics;
mod core;
mod data_views;
mod date;
mod disposable_stacks;
mod errors;
mod finalization_registries;
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
mod typed_arrays;
mod weak_maps;
mod weak_refs;
mod weak_sets;

use crate::CallEnv;
use crate::{Function, NativeFunction, RuntimeError, Value};

type NativeCallResult = Result<Option<Value>, RuntimeError>;

pub(crate) fn call_native_function(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: Vec<Value>,
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if matches!(native, NativeFunction::UninitializedLexical) {
        return Err(RuntimeError {
            thrown: None,
            message: "ReferenceError: uninitialized lexical binding".to_owned(),
        });
    }

    if let Some(value) = arrays::call_array_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) = array_buffers::call_array_buffer_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) = typed_arrays::call_typed_array_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) = data_views::call_data_view_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) = disposable_stacks::call_disposable_stack_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) = atomics::call_atomics_native(native, &argument_values, env)? {
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

    if let Some(value) = math::call_math_native(native, &argument_values, env)? {
        return Ok(value);
    }

    if let Some(value) = errors::call_error_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
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

    if let Some(value) = finalization_registries::call_finalization_registry_native(
        function,
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) = weak_refs::call_weak_ref_native(
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
        env,
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

    if let Some(value) = crate::iterator::call_iterator_native(
        native,
        this_value.clone(),
        &argument_values,
        is_construct,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) =
        crate::generator::call_generator_native(native, this_value.clone(), &argument_values, env)?
    {
        return Ok(value);
    }

    if let Some(value) =
        crate::async_function::call_async_await_native(function, native, &argument_values, env)?
    {
        return Ok(value);
    }

    if let Some(value) = crate::async_generator::call_async_generator_native(
        native,
        this_value.clone(),
        &argument_values,
        env,
    )? {
        return Ok(value);
    }

    if let Some(value) = crate::async_generator::call_async_generator_reaction(
        function,
        native,
        &argument_values,
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
