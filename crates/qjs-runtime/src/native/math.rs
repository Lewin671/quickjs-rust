use crate::{NativeFunction, math as math_builtins};

use super::NativeCallResult;

pub(super) fn call_math_native(
    native: NativeFunction,
    argument_values: &[crate::Value],
) -> NativeCallResult {
    let value = match native {
        NativeFunction::MathAbs => math_builtins::native_math_unary(argument_values, f64::abs)?,
        NativeFunction::MathAcos => math_builtins::native_math_unary(argument_values, f64::acos)?,
        NativeFunction::MathAcosh => math_builtins::native_math_unary(argument_values, f64::acosh)?,
        NativeFunction::MathAsin => math_builtins::native_math_unary(argument_values, f64::asin)?,
        NativeFunction::MathAsinh => math_builtins::native_math_unary(argument_values, f64::asinh)?,
        NativeFunction::MathAtan => math_builtins::native_math_unary(argument_values, f64::atan)?,
        NativeFunction::MathAtan2 => math_builtins::native_math_atan2(argument_values)?,
        NativeFunction::MathAtanh => math_builtins::native_math_unary(argument_values, f64::atanh)?,
        NativeFunction::MathCbrt => math_builtins::native_math_unary(argument_values, f64::cbrt)?,
        NativeFunction::MathCeil => math_builtins::native_math_unary(argument_values, f64::ceil)?,
        NativeFunction::MathClz32 => math_builtins::native_math_clz32(argument_values)?,
        NativeFunction::MathCos => math_builtins::native_math_unary(argument_values, f64::cos)?,
        NativeFunction::MathCosh => math_builtins::native_math_unary(argument_values, f64::cosh)?,
        NativeFunction::MathExp => math_builtins::native_math_unary(argument_values, f64::exp)?,
        NativeFunction::MathExpm1 => {
            math_builtins::native_math_unary(argument_values, f64::exp_m1)?
        }
        NativeFunction::MathFloor => math_builtins::native_math_unary(argument_values, f64::floor)?,
        NativeFunction::MathFround => math_builtins::native_math_fround(argument_values)?,
        NativeFunction::MathHypot => math_builtins::native_math_hypot(argument_values)?,
        NativeFunction::MathImul => math_builtins::native_math_imul(argument_values)?,
        NativeFunction::MathLog => math_builtins::native_math_unary(argument_values, f64::ln)?,
        NativeFunction::MathLog1p => math_builtins::native_math_unary(argument_values, f64::ln_1p)?,
        NativeFunction::MathLog10 => math_builtins::native_math_unary(argument_values, f64::log10)?,
        NativeFunction::MathLog2 => math_builtins::native_math_unary(argument_values, f64::log2)?,
        NativeFunction::MathMax => math_builtins::native_math_max(argument_values)?,
        NativeFunction::MathMin => math_builtins::native_math_min(argument_values)?,
        NativeFunction::MathPow => math_builtins::native_math_pow(argument_values)?,
        NativeFunction::MathRandom => math_builtins::native_math_random()?,
        NativeFunction::MathRound => math_builtins::native_math_round(argument_values)?,
        NativeFunction::MathSign => math_builtins::native_math_sign(argument_values)?,
        NativeFunction::MathSin => math_builtins::native_math_unary(argument_values, f64::sin)?,
        NativeFunction::MathSinh => math_builtins::native_math_unary(argument_values, f64::sinh)?,
        NativeFunction::MathSqrt => math_builtins::native_math_unary(argument_values, f64::sqrt)?,
        NativeFunction::MathTan => math_builtins::native_math_unary(argument_values, f64::tan)?,
        NativeFunction::MathTanh => math_builtins::native_math_unary(argument_values, f64::tanh)?,
        NativeFunction::MathTrunc => math_builtins::native_math_unary(argument_values, f64::trunc)?,
        _ => return Ok(None),
    };

    Ok(Some(value))
}
