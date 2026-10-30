//! Operation entries for objects; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, object};

pub(super) fn object(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.new_target.is_some(),
        env,
    )
}

pub(super) fn object_assign(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_assign(argument_values, env)
}

pub(super) fn object_create(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_create(argument_values, env)
}

pub(super) fn object_define_properties(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_define_properties(argument_values, env)
}

pub(super) fn object_define_property(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_define_property(argument_values, env)
}

pub(super) fn object_get_own_property_descriptor(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_get_own_property_descriptor(argument_values, env)
}

pub(super) fn object_get_own_property_descriptors(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_get_own_property_descriptors(argument_values, env)
}

pub(super) fn object_get_prototype_of(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_get_prototype_of(argument_values, env)
}

pub(super) fn object_get_own_property_names(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_get_own_property_names(argument_values, env)
}

pub(super) fn object_get_own_property_symbols(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_get_own_property_symbols(argument_values, env)
}

pub(super) fn object_from_entries(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_from_entries(argument_values, env)
}

pub(super) fn object_group_by(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_group_by(argument_values, env)
}

pub(super) fn object_freeze(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_freeze(argument_values, env)
}

pub(super) fn object_has_own(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_has_own(argument_values, env)
}

pub(super) fn object_is(
    _env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_is(argument_values)
}

pub(super) fn object_is_extensible(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_is_extensible(argument_values, env)
}

pub(super) fn object_is_frozen(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_is_frozen(argument_values, env)
}

pub(super) fn object_is_sealed(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_is_sealed(argument_values, env)
}

pub(super) fn object_prevent_extensions(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_prevent_extensions(argument_values, env)
}

pub(super) fn object_seal(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_seal(argument_values, env)
}

pub(super) fn object_set_prototype_of(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_set_prototype_of(argument_values, env)
}

pub(super) fn object_entries(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_entries(argument_values, env)
}

pub(super) fn object_keys(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_keys(argument_values, env)
}

pub(super) fn object_values(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_values(argument_values, env)
}

pub(super) fn object_prototype_has_own_property(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_prototype_has_own_property(info.this_value.clone(), argument_values, env)
}

pub(super) fn object_prototype_is_prototype_of(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_prototype_is_prototype_of(info.this_value.clone(), argument_values, env)
}

pub(super) fn object_prototype_property_is_enumerable(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_prototype_property_is_enumerable(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn object_prototype_to_string(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_prototype_to_string(info.this_value.clone(), env)
}

pub(super) fn object_prototype_to_locale_string(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_prototype_to_locale_string(info.this_value.clone(), env)
}

pub(super) fn object_prototype_value_of(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_prototype_value_of(info.this_value.clone(), env)
}

pub(super) fn object_prototype_get_proto(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_prototype_get_proto(info.this_value.clone(), env)
}

pub(super) fn object_prototype_set_proto(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_prototype_set_proto(info.this_value.clone(), argument_values, env)
}

pub(super) fn object_prototype_define_getter(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_prototype_define_getter(info.this_value.clone(), argument_values, env)
}

pub(super) fn object_prototype_define_setter(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_prototype_define_setter(info.this_value.clone(), argument_values, env)
}

pub(super) fn object_prototype_lookup_getter(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_prototype_lookup_getter(info.this_value.clone(), argument_values, env)
}

pub(super) fn object_prototype_lookup_setter(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    object::native_object_prototype_lookup_setter(info.this_value.clone(), argument_values, env)
}
