use std::collections::HashMap;

use num_bigint::BigInt;

use crate::{NativeFunction, RuntimeError, Value, to_number_with_env};

use super::{clamp_uint8, is_big_int_kind, modulo_integer, signed_integer};

/// Coerces an arbitrary value to the canonical element value for `native`,
/// applying the per-type numeric conversion (wrapping for integers, clamping
/// for `Uint8Clamped`, BigInt wrapping for the 64-bit kinds). The stored value
/// is always a `Number` (or `BigInt` for BigInt arrays).
pub(crate) fn coerce_element(
    native: NativeFunction,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if is_big_int_kind(native) {
        return coerce_big_int_element(native, value, env);
    }

    let number = to_number_with_env(value, env)?;
    let value = match native {
        NativeFunction::Uint8Array => modulo_integer(number, 256.0),
        NativeFunction::Int8Array => signed_integer(number, 8),
        NativeFunction::Uint8ClampedArray => clamp_uint8(number),
        NativeFunction::Uint16Array => modulo_integer(number, 65_536.0),
        NativeFunction::Int16Array => signed_integer(number, 16),
        NativeFunction::Uint32Array => modulo_integer(number, 4_294_967_296.0),
        NativeFunction::Int32Array => signed_integer(number, 32),
        NativeFunction::Float32Array => f32_round(number),
        NativeFunction::Float64Array => number,
        _ => unreachable!("non-bigint typed array native expected"),
    };
    Ok(Value::Number(value))
}

fn coerce_big_int_element(
    native: NativeFunction,
    value: Value,
    _env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    // ToBigInt: only BigInt and boolean coerce; numbers and the rest throw.
    let big = match value {
        Value::BigInt(value) => value,
        Value::Boolean(flag) => BigInt::from(u8::from(flag)),
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: cannot convert value to a BigInt typed array element"
                    .to_owned(),
            });
        }
    };
    Ok(Value::BigInt(wrap_big_int(native, big)))
}

fn wrap_big_int(native: NativeFunction, value: BigInt) -> BigInt {
    let modulo = BigInt::from(1u64) << 64;
    let mut wrapped = ((value % &modulo) + &modulo) % &modulo;
    if matches!(native, NativeFunction::BigInt64Array) {
        let sign = BigInt::from(1u64) << 63;
        if wrapped >= sign {
            wrapped -= &modulo;
        }
    }
    wrapped
}

/// Rounds a number to `f32` precision then back to `f64`, matching the storage
/// semantics of `Float32Array`.
fn f32_round(number: f64) -> f64 {
    f64::from(number as f32)
}
