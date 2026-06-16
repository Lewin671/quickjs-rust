use crate::{CallEnv, Function, NativeFunction, Value, disposable_stack};

use super::NativeCallResult;

pub(super) fn call_disposable_stack_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    is_construct: bool,
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::DisposableStack => {
            disposable_stack::native_disposable_stack(function, is_construct, env)?
        }
        NativeFunction::DisposableStackPrototypeDisposed => {
            disposable_stack::native_disposable_stack_prototype_disposed(this_value)?
        }
        NativeFunction::DisposableStackPrototypeDispose => {
            disposable_stack::native_disposable_stack_prototype_dispose(this_value)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
