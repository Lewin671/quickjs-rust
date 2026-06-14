use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::CallEnv;
use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value,
    array::array_like_values_with_env, symbol, to_number_with_env, to_uint32_number,
};

pub(super) fn install_math(env: &mut CallEnv, global_this: &Value, object_prototype: ObjectRef) {
    let math_object = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    math_object.set_to_string_tag("Math");
    symbol::define_well_known_to_string_tag(env, &math_object, "Math");
    define_math_constant(&math_object, "E", std::f64::consts::E);
    define_math_constant(&math_object, "LN10", std::f64::consts::LN_10);
    define_math_constant(&math_object, "LN2", std::f64::consts::LN_2);
    define_math_constant(&math_object, "LOG10E", std::f64::consts::LOG10_E);
    define_math_constant(&math_object, "LOG2E", std::f64::consts::LOG2_E);
    define_math_constant(&math_object, "PI", std::f64::consts::PI);
    define_math_constant(&math_object, "SQRT1_2", std::f64::consts::FRAC_1_SQRT_2);
    define_math_constant(&math_object, "SQRT2", std::f64::consts::SQRT_2);
    define_math_function(&math_object, "abs", 1, NativeFunction::MathAbs);
    define_math_function(&math_object, "acos", 1, NativeFunction::MathAcos);
    define_math_function(&math_object, "acosh", 1, NativeFunction::MathAcosh);
    define_math_function(&math_object, "asin", 1, NativeFunction::MathAsin);
    define_math_function(&math_object, "asinh", 1, NativeFunction::MathAsinh);
    define_math_function(&math_object, "atan", 1, NativeFunction::MathAtan);
    define_math_function(&math_object, "atan2", 2, NativeFunction::MathAtan2);
    define_math_function(&math_object, "atanh", 1, NativeFunction::MathAtanh);
    define_math_function(&math_object, "cbrt", 1, NativeFunction::MathCbrt);
    define_math_function(&math_object, "ceil", 1, NativeFunction::MathCeil);
    define_math_function(&math_object, "clz32", 1, NativeFunction::MathClz32);
    define_math_function(&math_object, "cos", 1, NativeFunction::MathCos);
    define_math_function(&math_object, "cosh", 1, NativeFunction::MathCosh);
    define_math_function(&math_object, "exp", 1, NativeFunction::MathExp);
    define_math_function(&math_object, "expm1", 1, NativeFunction::MathExpm1);
    define_math_function(&math_object, "f16round", 1, NativeFunction::MathF16round);
    define_math_function(&math_object, "floor", 1, NativeFunction::MathFloor);
    define_math_function(&math_object, "fround", 1, NativeFunction::MathFround);
    define_math_function(&math_object, "hypot", 2, NativeFunction::MathHypot);
    define_math_function(&math_object, "imul", 2, NativeFunction::MathImul);
    define_math_function(&math_object, "log", 1, NativeFunction::MathLog);
    define_math_function(&math_object, "log1p", 1, NativeFunction::MathLog1p);
    define_math_function(&math_object, "log10", 1, NativeFunction::MathLog10);
    define_math_function(&math_object, "log2", 1, NativeFunction::MathLog2);
    define_math_function(&math_object, "max", 2, NativeFunction::MathMax);
    define_math_function(&math_object, "min", 2, NativeFunction::MathMin);
    define_math_function(&math_object, "pow", 2, NativeFunction::MathPow);
    define_math_function(&math_object, "random", 0, NativeFunction::MathRandom);
    define_math_function(&math_object, "round", 1, NativeFunction::MathRound);
    define_math_function(&math_object, "sign", 1, NativeFunction::MathSign);
    define_math_function(&math_object, "sin", 1, NativeFunction::MathSin);
    define_math_function(&math_object, "sinh", 1, NativeFunction::MathSinh);
    define_math_function(&math_object, "sqrt", 1, NativeFunction::MathSqrt);
    define_math_function(
        &math_object,
        "sumPrecise",
        1,
        NativeFunction::MathSumPrecise,
    );
    define_math_function(&math_object, "tan", 1, NativeFunction::MathTan);
    define_math_function(&math_object, "tanh", 1, NativeFunction::MathTanh);
    define_math_function(&math_object, "trunc", 1, NativeFunction::MathTrunc);
    let math_value = Value::Object(math_object);
    env.insert_realm("Math".to_owned(), math_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("Math".to_owned(), math_value);
    }
}

fn define_math_constant(object: &ObjectRef, key: &str, value: f64) {
    object.define_property(
        key.to_owned(),
        Property::data(Value::Number(value), false, false, false),
    );
}

fn define_math_function(object: &ObjectRef, key: &str, length: usize, native: NativeFunction) {
    object.define_non_enumerable(
        key.to_owned(),
        Value::Function(Function::new_native(Some(key), length, native, false)),
    );
}

pub(super) fn native_math_unary(
    argument_values: &[Value],
    operation: fn(f64) -> f64,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let argument = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Number(operation(to_number_with_env(argument, env)?)))
}

pub(super) fn native_math_atan2(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let y = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let x = to_number_with_env(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    Ok(Value::Number(y.atan2(x)))
}

pub(super) fn native_math_fround(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let number = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    Ok(Value::Number(f64::from(number as f32)))
}

pub(super) fn native_math_f16round(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let number = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    Ok(Value::Number(round_to_binary16(number)))
}

pub(super) fn native_math_hypot(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let mut coerced = Vec::with_capacity(argument_values.len());
    for value in argument_values.iter().cloned() {
        coerced.push(to_number_with_env(value, env)?);
    }
    let mut sum = 0.0;
    let mut has_infinity = false;
    let mut has_nan = false;
    for number in coerced {
        if number.is_infinite() {
            has_infinity = true;
        } else if number.is_nan() {
            has_nan = true;
        } else {
            sum += number * number;
        }
    }
    if has_infinity {
        return Ok(Value::Number(f64::INFINITY));
    }
    if has_nan {
        return Ok(Value::Number(f64::NAN));
    }
    Ok(Value::Number(sum.sqrt()))
}

pub(super) fn native_math_max(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if argument_values.is_empty() {
        return Ok(Value::Number(f64::NEG_INFINITY));
    }

    let mut coerced = Vec::with_capacity(argument_values.len());
    for value in argument_values.iter().cloned() {
        coerced.push(to_number_with_env(value, env)?);
    }

    let mut maximum = f64::NEG_INFINITY;
    for number in coerced {
        if number.is_nan() {
            return Ok(Value::Number(f64::NAN));
        }
        if number > maximum || (number == 0.0 && maximum == 0.0 && number.is_sign_positive()) {
            maximum = number;
        }
    }
    Ok(Value::Number(maximum))
}

pub(super) fn native_math_min(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if argument_values.is_empty() {
        return Ok(Value::Number(f64::INFINITY));
    }

    let mut coerced = Vec::with_capacity(argument_values.len());
    for value in argument_values.iter().cloned() {
        coerced.push(to_number_with_env(value, env)?);
    }

    let mut minimum = f64::INFINITY;
    for number in coerced {
        if number.is_nan() {
            return Ok(Value::Number(f64::NAN));
        }
        if number < minimum || (number == 0.0 && minimum == 0.0 && number.is_sign_negative()) {
            minimum = number;
        }
    }
    Ok(Value::Number(minimum))
}

pub(super) fn native_math_pow(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let base = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let exponent = to_number_with_env(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if base.abs() == 1.0 && exponent.is_infinite() {
        return Ok(Value::Number(f64::NAN));
    }
    Ok(Value::Number(base.powf(exponent)))
}

pub(super) fn native_math_random() -> Result<Value, RuntimeError> {
    Ok(Value::Number(random_unit_interval()))
}

pub(super) fn native_math_round(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let number = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if number.is_nan() || number.is_infinite() || number == 0.0 {
        return Ok(Value::Number(number));
    }
    let floored = number.floor();
    let frac = number - floored;
    let rounded = if frac >= 0.5 {
        floored + 1.0
    } else {
        floored
    };
    if rounded == 0.0 && number < 0.0 {
        Ok(Value::Number(-0.0))
    } else {
        Ok(Value::Number(rounded))
    }
}

pub(super) fn native_math_sign(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let number = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if number.is_nan() || number == 0.0 {
        Ok(Value::Number(number))
    } else if number.is_sign_negative() {
        Ok(Value::Number(-1.0))
    } else {
        Ok(Value::Number(1.0))
    }
}

pub(super) fn native_math_clz32(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let number = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    Ok(Value::Number(f64::from(
        to_uint32_number(number).leading_zeros(),
    )))
}

pub(super) fn native_math_imul(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let left = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let right = to_number_with_env(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let product = to_uint32_number(left).wrapping_mul(to_uint32_number(right));
    Ok(Value::Number(f64::from(product as i32)))
}

pub(super) fn native_math_sum_precise(
    items: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let mut sum = SumPrecise::new();
    for value in array_like_values_with_env(items, "Math.sumPrecise", env)? {
        match value {
            Value::Number(number) => sum.add(number),
            _ => {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: Math.sumPrecise element must be a number".to_owned(),
                });
            }
        }
    }
    Ok(Value::Number(sum.result()))
}

const SUM_PRECISE_ACC_LEN: usize = 34;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SumPreciseState {
    MinusZero,
    Finite,
    Infinity,
    MinusInfinity,
    Nan,
}

struct SumPrecise {
    acc: [u64; SUM_PRECISE_ACC_LEN],
    n_limbs: usize,
    state: SumPreciseState,
}

impl SumPrecise {
    fn new() -> Self {
        let mut acc = [0; SUM_PRECISE_ACC_LEN];
        acc[0] = 0;
        Self {
            acc,
            n_limbs: 1,
            state: SumPreciseState::MinusZero,
        }
    }

    fn add(&mut self, number: f64) {
        let bits = number.to_bits();
        let sign = bits >> 63;
        let exponent = ((bits >> 52) & ((1 << 11) - 1)) as i32;
        let mut mantissa = bits & ((1_u64 << 52) - 1);

        if exponent == 2047 {
            if mantissa == 0 {
                self.add_infinity(sign != 0);
            } else {
                self.state = SumPreciseState::Nan;
            }
            return;
        }

        if exponent == 0 {
            if mantissa == 0 {
                if self.state == SumPreciseState::MinusZero && sign == 0 {
                    self.state = SumPreciseState::Finite;
                }
                return;
            }
            self.add_finite(sign, 0, 0, mantissa);
            return;
        }

        mantissa |= 1_u64 << 52;
        let shift = (exponent - 1) as usize;
        self.add_finite(sign, shift / 64, shift % 64, mantissa);
    }

    fn add_infinity(&mut self, is_negative: bool) {
        self.state = match (self.state, is_negative) {
            (SumPreciseState::Nan, _) => SumPreciseState::Nan,
            (SumPreciseState::MinusInfinity, false) => SumPreciseState::Nan,
            (SumPreciseState::Infinity, true) => SumPreciseState::Nan,
            (_, true) => SumPreciseState::MinusInfinity,
            (_, false) => SumPreciseState::Infinity,
        };
    }

    fn add_finite(&mut self, sign: u64, mut limb_index: usize, shift: usize, mantissa: u64) {
        if matches!(
            self.state,
            SumPreciseState::Infinity | SumPreciseState::MinusInfinity | SumPreciseState::Nan
        ) {
            return;
        }
        self.state = SumPreciseState::Finite;

        let mut n = self.n_limbs;
        let acc_sign = sign_extend(self.acc[n - 1]);
        for index in n..=limb_index {
            self.acc[index] = acc_sign;
        }

        let mut carry = sign;
        let add_sign = 0_u64.wrapping_sub(sign);
        let low = mantissa << shift;
        (self.acc[limb_index], carry) = add_with_carry(self.acc[limb_index], low ^ add_sign, carry);

        if shift >= 12 {
            limb_index += 1;
            if limb_index >= n {
                self.acc[limb_index] = acc_sign;
            }
            let high = mantissa >> (64 - shift);
            (self.acc[limb_index], carry) =
                add_with_carry(self.acc[limb_index], high ^ add_sign, carry);
        }

        limb_index += 1;
        if limb_index >= n {
            n = limb_index;
        } else {
            for index in limb_index..n {
                if carry == sign {
                    self.n_limbs = n;
                    return;
                }
                (self.acc[index], carry) = add_with_carry(self.acc[index], add_sign, carry);
            }
        }

        let extension = carry.wrapping_add(acc_sign).wrapping_add(add_sign);
        if extension != sign_extend(self.acc[n - 1]) {
            self.acc[n] = extension;
            n += 1;
        }
        self.n_limbs = n;
    }

    fn result(&mut self) -> f64 {
        match self.state {
            SumPreciseState::MinusZero => return -0.0,
            SumPreciseState::Infinity => return f64::INFINITY,
            SumPreciseState::MinusInfinity => return f64::NEG_INFINITY,
            SumPreciseState::Nan => return f64::NAN,
            SumPreciseState::Finite => {}
        }

        let mut n = self.n_limbs;
        let is_negative = self.acc[n - 1] >> 63;
        if is_negative != 0 {
            let mut carry = 1;
            for index in 0..n {
                (self.acc[index], carry) = add_with_carry(!self.acc[index], 0, carry);
            }
        }

        while n > 0 && self.acc[n - 1] == 0 {
            n -= 1;
        }
        if n == 0 {
            return 0.0;
        }
        if n == 1 && self.acc[0] < (1_u64 << 52) {
            return f64::from_bits((is_negative << 63) | self.acc[0]);
        }

        let mut exponent = (n * 64) as i32;
        let mut limb_index = n - 1;
        let mut mantissa = self.acc[limb_index];
        let shift = mantissa.leading_zeros() as usize;
        exponent = exponent - shift as i32 - 52;
        if shift != 0 {
            mantissa <<= shift;
            if limb_index > 0 {
                limb_index -= 1;
                let remaining_shift = 64 - shift;
                let non_zero_mask = (1_u64 << remaining_shift) - 1;
                let non_zero = (self.acc[limb_index] & non_zero_mask) != 0;
                mantissa =
                    mantissa | (self.acc[limb_index] >> remaining_shift) | u64::from(non_zero);
            }
        }

        if (mantissa & ((1_u64 << 10) - 1)) == 0 {
            while limb_index > 0 {
                limb_index -= 1;
                if self.acc[limb_index] != 0 {
                    mantissa |= 1;
                    break;
                }
            }
        }

        let addend = (1_u64 << 10) - 1 + ((mantissa >> 11) & 1);
        mantissa = mantissa.wrapping_add(addend) >> 11;
        if mantissa == 0 {
            exponent += 1;
        }
        if exponent >= 2047 {
            f64::from_bits((is_negative << 63) | (2047_u64 << 52))
        } else {
            f64::from_bits(
                (is_negative << 63) | ((exponent as u64) << 52) | (mantissa & ((1_u64 << 52) - 1)),
            )
        }
    }
}

fn add_with_carry(left: u64, right: u64, carry_in: u64) -> (u64, u64) {
    let (sum, carry_left) = left.overflowing_add(right);
    let (sum, carry_right) = sum.overflowing_add(carry_in);
    (sum, u64::from(carry_left || carry_right))
}

fn sign_extend(value: u64) -> u64 {
    ((value as i64) >> 63) as u64
}

static RANDOM_STATE: AtomicU64 = AtomicU64::new(0);

fn random_unit_interval() -> f64 {
    let value = next_random_u64();
    ((value >> 11) as f64) * (1.0 / ((1_u64 << 53) as f64))
}

fn next_random_u64() -> u64 {
    let mut state = current_random_state();
    loop {
        let next = xorshift64star(state);
        match RANDOM_STATE.compare_exchange(state, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return next,
            Err(actual) => state = actual,
        }
    }
}

fn current_random_state() -> u64 {
    let state = RANDOM_STATE.load(Ordering::Acquire);
    if state != 0 {
        return state;
    }

    let seed = random_seed();
    match RANDOM_STATE.compare_exchange(0, seed, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => seed,
        Err(actual) => actual,
    }
}

fn random_seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0x9e37_79b9_7f4a_7c15);
    if nanos == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        nanos
    }
}

fn xorshift64star(mut value: u64) -> u64 {
    value ^= value >> 12;
    value ^= value << 25;
    value ^= value >> 27;
    value.wrapping_mul(0x2545_f491_4f6c_dd1d)
}

fn round_to_binary16(number: f64) -> f64 {
    if number.is_nan() || number.is_infinite() || number == 0.0 {
        return number;
    }

    let sign = if number.is_sign_negative() { -1.0 } else { 1.0 };
    let magnitude = number.abs();
    if magnitude >= 65520.0 {
        return sign * f64::INFINITY;
    }

    const MIN_NORMAL: f64 = 0.00006103515625; // 2^-14
    const MIN_SUBNORMAL: f64 = 0.000_000_059_604_644_775_390_63; // 2^-24

    if magnitude < MIN_NORMAL {
        return sign * (round_ties_to_even(magnitude / MIN_SUBNORMAL) * MIN_SUBNORMAL);
    }

    let exponent = magnitude.log2().floor();
    let unit = 2.0_f64.powf(exponent - 10.0);
    let significand = round_ties_to_even(magnitude / unit);
    if significand == 2048.0 {
        sign * 2.0_f64.powf(exponent + 1.0)
    } else {
        sign * significand * unit
    }
}

fn round_ties_to_even(value: f64) -> f64 {
    let lower = value.floor();
    let fraction = value - lower;
    if fraction < 0.5 {
        lower
    } else if fraction > 0.5 {
        lower + 1.0
    } else if lower % 2.0 == 0.0 {
        lower
    } else {
        lower + 1.0
    }
}
