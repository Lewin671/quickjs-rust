use crate::{Function, NativeFunction, Value, finalization_registry};

use super::NativeCallResult;
use crate::CallEnv;

pub(super) fn call_finalization_registry_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::FinalizationRegistry => {
            finalization_registry::native_finalization_registry(
                function,
                argument_values,
                is_construct,
                env,
            )?
        }
        NativeFunction::FinalizationRegistryPrototypeRegister => {
            finalization_registry::native_finalization_registry_prototype_register(
                this_value,
                argument_values,
                env,
            )?
        }
        NativeFunction::FinalizationRegistryPrototypeUnregister => {
            finalization_registry::native_finalization_registry_prototype_unregister(
                this_value,
                argument_values,
                env,
            )?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
