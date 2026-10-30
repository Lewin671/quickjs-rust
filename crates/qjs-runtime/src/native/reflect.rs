//! Operation entries for reflect; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, reflect};

pub(super) fn reflect_apply(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_apply(argument_values, env)
}

pub(super) fn reflect_construct(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_construct(argument_values, env)
}

pub(super) fn reflect_define_property(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_define_property(argument_values, env)
}

pub(super) fn reflect_delete_property(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_delete_property(argument_values, env)
}

pub(super) fn reflect_get(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_get(argument_values, env)
}

pub(super) fn reflect_get_prototype_of(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_get_prototype_of(argument_values, env)
}

pub(super) fn reflect_get_own_property_descriptor(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_get_own_property_descriptor(argument_values, env)
}

pub(super) fn reflect_has(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_has(argument_values, env)
}

pub(super) fn reflect_is_extensible(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_is_extensible(argument_values, env)
}

pub(super) fn reflect_own_keys(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_own_keys(argument_values, env)
}

pub(super) fn reflect_prevent_extensions(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_prevent_extensions(argument_values, env)
}

pub(super) fn reflect_set(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_set(argument_values, env)
}

pub(super) fn reflect_set_prototype_of(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_set_prototype_of(argument_values, env)
}
