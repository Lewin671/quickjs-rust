//! Operation entries for regexp; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, regexp};

pub(super) fn reg_exp(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn reg_exp_escape(
    _env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_escape(argument_values)
}

pub(super) fn reg_exp_prototype_compile(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_compile(info.this_value.clone(), argument_values, env)
}

pub(super) fn reg_exp_prototype_dot_all(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_flag(info.callee, info.this_value.clone(), env, 's')
}

pub(super) fn reg_exp_prototype_exec(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_exec(info.this_value.clone(), argument_values, env)
}

pub(super) fn reg_exp_prototype_flags(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_flags(info.callee, info.this_value.clone(), env)
}

pub(super) fn reg_exp_prototype_global(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_flag(info.callee, info.this_value.clone(), env, 'g')
}

pub(super) fn reg_exp_prototype_has_indices(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_flag(info.callee, info.this_value.clone(), env, 'd')
}

pub(super) fn reg_exp_prototype_ignore_case(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_flag(info.callee, info.this_value.clone(), env, 'i')
}

pub(super) fn reg_exp_prototype_match(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_match(info.this_value.clone(), argument_values, env)
}

pub(super) fn reg_exp_prototype_match_all(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_match_all(info.this_value.clone(), argument_values, env)
}

pub(super) fn reg_exp_prototype_multiline(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_flag(info.callee, info.this_value.clone(), env, 'm')
}

pub(super) fn reg_exp_prototype_replace(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_replace(info.this_value.clone(), argument_values, env)
}

pub(super) fn reg_exp_prototype_search(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_search(info.this_value.clone(), argument_values, env)
}

pub(super) fn reg_exp_prototype_split(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_split(info.this_value.clone(), argument_values, env)
}

pub(super) fn reg_exp_prototype_source(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_source(info.callee, info.this_value.clone(), env)
}

pub(super) fn reg_exp_prototype_sticky(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_flag(info.callee, info.this_value.clone(), env, 'y')
}

pub(super) fn reg_exp_prototype_test(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_test(info.this_value.clone(), argument_values, env)
}

pub(super) fn reg_exp_prototype_to_string(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_to_string(info.this_value.clone())
}

pub(super) fn reg_exp_prototype_unicode(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_flag(info.callee, info.this_value.clone(), env, 'u')
}

pub(super) fn reg_exp_prototype_unicode_sets(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_prototype_flag(info.callee, info.this_value.clone(), env, 'v')
}

pub(super) fn reg_exp_string_iterator_prototype_next(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    regexp::native_regexp_string_iterator_next(info.this_value.clone(), env)
}
