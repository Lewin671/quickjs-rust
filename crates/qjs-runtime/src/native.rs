#[cfg(feature = "agents")]
mod agent;
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
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if matches!(native, NativeFunction::UninitializedLexical) {
        return Err(RuntimeError {
            thrown: None,
            message: "ReferenceError: uninitialized lexical binding".to_owned(),
        });
    }
    // Each family's dispatcher answers `None` for a native it does not own,
    // so asking them in turn finds the owner; the owner is remembered on the
    // function object, and later calls ask it first.
    let known = function.native_family.get();
    if known != 0
        && let Some(value) = call_native_family(
            known,
            function,
            native,
            &this_value,
            argument_values,
            is_construct,
            env,
        )?
    {
        return Ok(value);
    }
    for family in 1..=NATIVE_FAMILIES {
        if family == known {
            continue;
        }
        if let Some(value) = call_native_family(
            family,
            function,
            native,
            &this_value,
            argument_values,
            is_construct,
            env,
        )? {
            function.native_family.set(family);
            return Ok(value);
        }
    }
    core::call_core_native(
        function,
        native,
        this_value,
        argument_values,
        is_construct,
        env,
    )
}

/// The number of families [`call_native_family`] asks.
const NATIVE_FAMILIES: u8 = 28;

/// Asks one builtin family's dispatcher to answer `native`.
fn call_native_family(
    family: u8,
    function: &Function,
    native: NativeFunction,
    this_value: &Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> NativeCallResult {
    match family {
        1 => arrays::call_array_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        2 => array_buffers::call_array_buffer_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        3 => typed_arrays::call_typed_array_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        4 => data_views::call_data_view_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        5 => disposable_stacks::call_disposable_stack_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        6 => atomics::call_atomics_native(native, argument_values, env),
        #[cfg(feature = "agents")]
        7 => agent::call_agent_native(native, argument_values, env),
        8 => date::call_date_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        9 => math::call_math_native(native, argument_values, env),
        10 => errors::call_error_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        11 => json::call_json_native(native, argument_values, env),
        12 => maps::call_map_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        13 => sets::call_set_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        14 => weak_maps::call_weak_map_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        15 => finalization_registries::call_finalization_registry_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        16 => weak_refs::call_weak_ref_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        17 => weak_sets::call_weak_set_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        18 => numbers::call_number_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        19 => promises::call_promise_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        20 => objects::call_object_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        21 => reflect::call_reflect_native(native, argument_values, env),
        22 => regexp::call_regexp_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        23 => strings::call_string_native(
            function,
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        24 => crate::iterator::call_iterator_native(
            native,
            this_value.clone(),
            argument_values,
            is_construct,
            env,
        ),
        25 => crate::generator::call_generator_native(
            native,
            this_value.clone(),
            argument_values,
            env,
        ),
        26 => {
            crate::async_function::call_async_await_native(function, native, argument_values, env)
        }
        27 => crate::async_generator::call_async_generator_native(
            native,
            this_value.clone(),
            argument_values,
            env,
        ),
        28 => crate::async_generator::call_async_generator_reaction(
            function,
            native,
            argument_values,
            env,
        ),
        _ => Ok(None),
    }
}
