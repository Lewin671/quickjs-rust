use crate::{CallEnv, Function, NativeFunction, Value, disposable_stack};

use super::NativeCallResult;

pub(super) fn call_disposable_stack_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::AsyncDisposableStack => {
            disposable_stack::native_async_disposable_stack(function, is_construct, env)?
        }
        NativeFunction::DisposableStack => {
            disposable_stack::native_disposable_stack(function, is_construct, env)?
        }
        NativeFunction::DisposableStackPrototypeAdopt => {
            disposable_stack::native_disposable_stack_prototype_adopt(this_value, argument_values)?
        }
        NativeFunction::DisposableStackPrototypeDefer => {
            disposable_stack::native_disposable_stack_prototype_defer(this_value, argument_values)?
        }
        NativeFunction::DisposableStackPrototypeDisposed => {
            disposable_stack::native_disposable_stack_prototype_disposed(this_value)?
        }
        NativeFunction::DisposableStackPrototypeDispose => {
            disposable_stack::native_disposable_stack_prototype_dispose(this_value, env)?
        }
        NativeFunction::DisposableStackPrototypeUse => {
            disposable_stack::native_disposable_stack_prototype_use(
                this_value,
                argument_values,
                env,
            )?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
