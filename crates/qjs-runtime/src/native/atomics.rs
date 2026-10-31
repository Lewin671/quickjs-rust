//! Operation entries for atomics; bound once when a native is created.

use super::CallInfo;
use crate::{RuntimeError, Value, atomics};

pub(super) fn atomics_add(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    atomics::native_atomics_read_modify_write(argument_values, atomics::AtomicOp::Add, env)
}

pub(super) fn atomics_and(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    atomics::native_atomics_read_modify_write(argument_values, atomics::AtomicOp::And, env)
}

pub(super) fn atomics_compare_exchange(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    atomics::native_atomics_compare_exchange(argument_values, env)
}

pub(super) fn atomics_exchange(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    atomics::native_atomics_read_modify_write(argument_values, atomics::AtomicOp::Exchange, env)
}

pub(super) fn atomics_is_lock_free(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    atomics::native_atomics_is_lock_free(argument_values, env)
}

pub(super) fn atomics_load(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    atomics::native_atomics_load(argument_values, env)
}

pub(super) fn atomics_notify(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    atomics::native_atomics_notify(argument_values, env)
}

pub(super) fn atomics_or(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    atomics::native_atomics_read_modify_write(argument_values, atomics::AtomicOp::Or, env)
}

pub(super) fn atomics_pause(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    atomics::native_atomics_pause(argument_values)
}

pub(super) fn atomics_store(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    atomics::native_atomics_store(argument_values, env)
}

pub(super) fn atomics_sub(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    atomics::native_atomics_read_modify_write(argument_values, atomics::AtomicOp::Sub, env)
}

pub(super) fn atomics_wait(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    atomics::native_atomics_wait(argument_values, env)
}

pub(super) fn atomics_xor(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    atomics::native_atomics_read_modify_write(argument_values, atomics::AtomicOp::Xor, env)
}
