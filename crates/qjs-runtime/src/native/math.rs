//! Operation entries for math; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, math as math_builtins};

pub(super) fn math_abs(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::abs, env)
}

pub(super) fn math_acos(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::acos, env)
}

pub(super) fn math_acosh(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::acosh, env)
}

pub(super) fn math_asin(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::asin, env)
}

pub(super) fn math_asinh(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::asinh, env)
}

pub(super) fn math_atan(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::atan, env)
}

pub(super) fn math_atan2(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_atan2(argument_values, env)
}

pub(super) fn math_atanh(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::atanh, env)
}

pub(super) fn math_cbrt(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::cbrt, env)
}

pub(super) fn math_ceil(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::ceil, env)
}

pub(super) fn math_clz32(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_clz32(argument_values, env)
}

pub(super) fn math_cos(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::cos, env)
}

pub(super) fn math_cosh(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::cosh, env)
}

pub(super) fn math_exp(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::exp, env)
}

pub(super) fn math_expm1(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::exp_m1, env)
}

pub(super) fn math_f16round(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_f16round(argument_values, env)
}

pub(super) fn math_floor(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::floor, env)
}

pub(super) fn math_fround(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_fround(argument_values, env)
}

pub(super) fn math_hypot(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_hypot(argument_values, env)
}

pub(super) fn math_imul(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_imul(argument_values, env)
}

pub(super) fn math_log(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::ln, env)
}

pub(super) fn math_log1p(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::ln_1p, env)
}

pub(super) fn math_log10(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::log10, env)
}

pub(super) fn math_log2(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::log2, env)
}

pub(super) fn math_max(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_max(argument_values, env)
}

pub(super) fn math_min(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_min(argument_values, env)
}

pub(super) fn math_pow(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_pow(argument_values, env)
}

pub(super) fn math_random(
    _env: &mut CallEnv,
    _info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_random()
}

pub(super) fn math_round(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_round(argument_values, env)
}

pub(super) fn math_sign(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_sign(argument_values, env)
}

pub(super) fn math_sin(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::sin, env)
}

pub(super) fn math_sinh(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::sinh, env)
}

pub(super) fn math_sqrt(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::sqrt, env)
}

pub(super) fn math_sum_precise(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_sum_precise(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )
}

pub(super) fn math_tan(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::tan, env)
}

pub(super) fn math_tanh(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::tanh, env)
}

pub(super) fn math_trunc(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    math_builtins::native_math_unary(argument_values, f64::trunc, env)
}
