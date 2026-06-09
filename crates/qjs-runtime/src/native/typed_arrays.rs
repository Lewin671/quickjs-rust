use std::collections::HashMap;

use crate::{Function, NativeFunction, Value, typed_array};

use super::NativeCallResult;

pub(super) fn call_typed_array_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::Uint8Array
        | NativeFunction::Uint16Array
        | NativeFunction::Uint32Array
        | NativeFunction::Float32Array
        | NativeFunction::Float64Array => typed_array::native_typed_array(
            function,
            native,
            this_value,
            argument_values,
            is_construct,
            env,
        )?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}
