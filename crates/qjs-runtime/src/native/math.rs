use crate::{NativeFunction, Value, math as math_builtins};

use super::NativeCallResult;
use crate::CallEnv;

pub(super) fn call_math_native(
    native: NativeFunction,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::MathAbs => {
            math_builtins::native_math_unary(argument_values, f64::abs, env)?
        }
        NativeFunction::MathAcos => {
            math_builtins::native_math_unary(argument_values, f64::acos, env)?
        }
        NativeFunction::MathAcosh => {
            math_builtins::native_math_unary(argument_values, f64::acosh, env)?
        }
        NativeFunction::MathAsin => {
            math_builtins::native_math_unary(argument_values, f64::asin, env)?
        }
        NativeFunction::MathAsinh => {
            math_builtins::native_math_unary(argument_values, f64::asinh, env)?
        }
        NativeFunction::MathAtan => {
            math_builtins::native_math_unary(argument_values, f64::atan, env)?
        }
        NativeFunction::MathAtan2 => math_builtins::native_math_atan2(argument_values, env)?,
        NativeFunction::MathAtanh => {
            math_builtins::native_math_unary(argument_values, f64::atanh, env)?
        }
        NativeFunction::MathCbrt => {
            math_builtins::native_math_unary(argument_values, f64::cbrt, env)?
        }
        NativeFunction::MathCeil => {
            math_builtins::native_math_unary(argument_values, f64::ceil, env)?
        }
        NativeFunction::MathClz32 => math_builtins::native_math_clz32(argument_values, env)?,
        NativeFunction::MathCos => {
            math_builtins::native_math_unary(argument_values, f64::cos, env)?
        }
        NativeFunction::MathCosh => {
            math_builtins::native_math_unary(argument_values, f64::cosh, env)?
        }
        NativeFunction::MathExp => {
            math_builtins::native_math_unary(argument_values, f64::exp, env)?
        }
        NativeFunction::MathExpm1 => {
            math_builtins::native_math_unary(argument_values, f64::exp_m1, env)?
        }
        NativeFunction::MathF16round => math_builtins::native_math_f16round(argument_values, env)?,
        NativeFunction::MathFloor => {
            math_builtins::native_math_unary(argument_values, f64::floor, env)?
        }
        NativeFunction::MathFround => math_builtins::native_math_fround(argument_values, env)?,
        NativeFunction::MathHypot => math_builtins::native_math_hypot(argument_values, env)?,
        NativeFunction::MathImul => math_builtins::native_math_imul(argument_values, env)?,
        NativeFunction::MathLog => math_builtins::native_math_unary(argument_values, f64::ln, env)?,
        NativeFunction::MathLog1p => {
            math_builtins::native_math_unary(argument_values, f64::ln_1p, env)?
        }
        NativeFunction::MathLog10 => {
            math_builtins::native_math_unary(argument_values, f64::log10, env)?
        }
        NativeFunction::MathLog2 => {
            math_builtins::native_math_unary(argument_values, f64::log2, env)?
        }
        NativeFunction::MathMax => math_builtins::native_math_max(argument_values, env)?,
        NativeFunction::MathMin => math_builtins::native_math_min(argument_values, env)?,
        NativeFunction::MathPow => math_builtins::native_math_pow(argument_values, env)?,
        NativeFunction::MathRandom => math_builtins::native_math_random()?,
        NativeFunction::MathRound => math_builtins::native_math_round(argument_values, env)?,
        NativeFunction::MathSign => math_builtins::native_math_sign(argument_values, env)?,
        NativeFunction::MathSin => {
            math_builtins::native_math_unary(argument_values, f64::sin, env)?
        }
        NativeFunction::MathSinh => {
            math_builtins::native_math_unary(argument_values, f64::sinh, env)?
        }
        NativeFunction::MathSqrt => {
            math_builtins::native_math_unary(argument_values, f64::sqrt, env)?
        }
        NativeFunction::MathSumPrecise => math_builtins::native_math_sum_precise(
            argument_values.first().cloned().unwrap_or(Value::Undefined),
            env,
        )?,
        NativeFunction::MathTan => {
            math_builtins::native_math_unary(argument_values, f64::tan, env)?
        }
        NativeFunction::MathTanh => {
            math_builtins::native_math_unary(argument_values, f64::tanh, env)?
        }
        NativeFunction::MathTrunc => {
            math_builtins::native_math_unary(argument_values, f64::trunc, env)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
