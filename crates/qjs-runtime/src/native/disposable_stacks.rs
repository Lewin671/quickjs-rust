//! Operation entries for disposable stacks; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, disposable_stack};

pub(super) fn async_disposable_stack(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    disposable_stack::native_async_disposable_stack(info.callee, info.is_construct, env)
}

pub(super) fn async_disposable_stack_prototype_adopt(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    disposable_stack::native_async_disposable_stack_prototype_adopt(
        info.this_value.clone(),
        argument_values,
    )
}

pub(super) fn async_disposable_stack_prototype_defer(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    disposable_stack::native_async_disposable_stack_prototype_defer(
        info.this_value.clone(),
        argument_values,
    )
}

pub(super) fn async_disposable_stack_prototype_disposed(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    disposable_stack::native_async_disposable_stack_prototype_disposed(info.this_value.clone())
}

pub(super) fn async_disposable_stack_prototype_dispose_async(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    disposable_stack::native_async_disposable_stack_prototype_dispose_async(
        info.this_value.clone(),
        env,
    )
}

pub(super) fn async_disposable_stack_prototype_move(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    disposable_stack::native_async_disposable_stack_prototype_move(info.this_value.clone(), env)
}

pub(super) fn async_disposable_stack_prototype_use(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    disposable_stack::native_async_disposable_stack_prototype_use(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn disposable_stack(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    disposable_stack::native_disposable_stack(info.callee, info.is_construct, env)
}

pub(super) fn disposable_stack_prototype_adopt(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    disposable_stack::native_disposable_stack_prototype_adopt(
        info.this_value.clone(),
        argument_values,
    )
}

pub(super) fn disposable_stack_prototype_defer(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    disposable_stack::native_disposable_stack_prototype_defer(
        info.this_value.clone(),
        argument_values,
    )
}

pub(super) fn disposable_stack_prototype_disposed(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    disposable_stack::native_disposable_stack_prototype_disposed(info.this_value.clone())
}

pub(super) fn disposable_stack_prototype_dispose(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    disposable_stack::native_disposable_stack_prototype_dispose(info.this_value.clone(), env)
}

pub(super) fn disposable_stack_prototype_move(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    disposable_stack::native_disposable_stack_prototype_move(info.this_value.clone(), env)
}

pub(super) fn disposable_stack_prototype_use(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    disposable_stack::native_disposable_stack_prototype_use(
        info.this_value.clone(),
        argument_values,
        env,
    )
}
