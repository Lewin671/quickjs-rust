//! Operation entries for reflect; bound once when a native is created.

use super::CallInfo;
use crate::{RuntimeError, Value, reflect};

pub(super) fn reflect_apply(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_apply(argument_values, env)
}

pub(super) fn reflect_construct(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_construct(argument_values, env)
}

pub(super) fn reflect_define_property(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_define_property(argument_values, env)
}

pub(super) fn reflect_delete_property(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_delete_property(argument_values, env)
}

pub(super) fn reflect_get(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_get(argument_values, env)
}

pub(super) fn reflect_get_prototype_of(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_get_prototype_of(argument_values, env)
}

pub(super) fn reflect_get_own_property_descriptor(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_get_own_property_descriptor(argument_values, env)
}

pub(super) fn reflect_has(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_has(argument_values, env)
}

pub(super) fn reflect_is_extensible(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_is_extensible(argument_values, env)
}

pub(super) fn reflect_own_keys(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_own_keys(argument_values, env)
}

pub(super) fn reflect_prevent_extensions(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_prevent_extensions(argument_values, env)
}

pub(super) fn reflect_set(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_set(argument_values, env)
}

pub(super) fn reflect_set_prototype_of(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    reflect::native_reflect_set_prototype_of(argument_values, env)
}
