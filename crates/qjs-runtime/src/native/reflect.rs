use crate::{NativeFunction, Value, reflect};

use super::NativeCallResult;
use crate::CallEnv;

pub(super) fn call_reflect_native(
    native: NativeFunction,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::ReflectApply => reflect::native_reflect_apply(argument_values, env)?,
        NativeFunction::ReflectConstruct => {
            reflect::native_reflect_construct(argument_values, env)?
        }
        NativeFunction::ReflectDefineProperty => {
            reflect::native_reflect_define_property(argument_values, env)?
        }
        NativeFunction::ReflectDeleteProperty => {
            reflect::native_reflect_delete_property(argument_values, env)?
        }
        NativeFunction::ReflectGet => reflect::native_reflect_get(argument_values, env)?,
        NativeFunction::ReflectGetPrototypeOf => {
            reflect::native_reflect_get_prototype_of(argument_values, env)?
        }
        NativeFunction::ReflectGetOwnPropertyDescriptor => {
            reflect::native_reflect_get_own_property_descriptor(argument_values, env)?
        }
        NativeFunction::ReflectHas => reflect::native_reflect_has(argument_values, env)?,
        NativeFunction::ReflectIsExtensible => {
            reflect::native_reflect_is_extensible(argument_values, env)?
        }
        NativeFunction::ReflectOwnKeys => reflect::native_reflect_own_keys(argument_values, env)?,
        NativeFunction::ReflectPreventExtensions => {
            reflect::native_reflect_prevent_extensions(argument_values, env)?
        }
        NativeFunction::ReflectSet => reflect::native_reflect_set(argument_values, env)?,
        NativeFunction::ReflectSetPrototypeOf => {
            reflect::native_reflect_set_prototype_of(argument_values, env)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
