use std::collections::HashMap;

use crate::{NativeFunction, Value, reflect};

use super::NativeCallResult;

pub(super) fn call_reflect_native(
    native: NativeFunction,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::ReflectGetPrototypeOf => {
            reflect::native_reflect_get_prototype_of(argument_values, env)?
        }
        NativeFunction::ReflectGetOwnPropertyDescriptor => {
            reflect::native_reflect_get_own_property_descriptor(argument_values, env)?
        }
        NativeFunction::ReflectHas => reflect::native_reflect_has(argument_values, env)?,
        NativeFunction::ReflectOwnKeys => reflect::native_reflect_own_keys(argument_values)?,
        NativeFunction::ReflectSetPrototypeOf => {
            reflect::native_reflect_set_prototype_of(argument_values)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
