use std::cmp::Ordering;

use num_bigint::{BigInt, Sign};
use num_traits::{ToPrimitive, Zero};
use qjs_ast::{BinaryOp, UnaryOp};

use crate::CallEnv;
use crate::{
    PreferredType, PropertyKey, RuntimeError, Value, call_function, error, has_property_key,
    is_truthy, property_value, string, symbol, to_int32_number, to_js_string_with_env,
    to_number_with_env, to_primitive_with_env, to_primitive_with_hint, to_property_key_value,
    to_uint32_number, value_prototype_slot,
};

pub(crate) fn eval_unary(
    op: UnaryOp,
    argument: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    match op {
        UnaryOp::Not => Ok(Value::Boolean(!is_truthy(&argument))),
        UnaryOp::Plus => Ok(Value::Number(to_number_with_env(argument, env)?)),
        // Unary `-` and `~` coerce with ToNumeric, so an operand that becomes a
        // BigInt (including a wrapper object or a Symbol.toPrimitive that
        // returns a BigInt) uses the BigInt operation rather than ToNumber,
        // which would throw.
        UnaryOp::Minus => match to_numeric_with_env(argument, env)? {
            Value::BigInt(value) => Ok(Value::BigInt(-value)),
            Value::Number(value) => Ok(Value::Number(-value)),
            _ => unreachable!("ToNumeric yields a Number or BigInt"),
        },
        UnaryOp::BitwiseNot => match to_numeric_with_env(argument, env)? {
            Value::BigInt(value) => Ok(Value::BigInt(!value)),
            Value::Number(value) => Ok(Value::Number(f64::from(!to_int32_number(value)))),
            _ => unreachable!("ToNumeric yields a Number or BigInt"),
        },
        UnaryOp::Void => Ok(Value::Undefined),
        UnaryOp::Typeof | UnaryOp::Delete => {
            unreachable!("operator requires unevaluated operand handling")
        }
    }
}

pub(crate) fn eval_binary(
    left: Value,
    op: BinaryOp,
    right: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if op == BinaryOp::In {
        return eval_in(left, right, env);
    }
    if op == BinaryOp::Instanceof {
        return eval_instanceof(left, right, env);
    }

    match op {
        BinaryOp::Eq => return Ok(Value::Boolean(abstract_eq(&left, &right, env)?)),
        BinaryOp::Ne => return Ok(Value::Boolean(!abstract_eq(&left, &right, env)?)),
        BinaryOp::StrictEq => return Ok(Value::Boolean(strict_eq(&left, &right))),
        BinaryOp::StrictNe => return Ok(Value::Boolean(!strict_eq(&left, &right))),
        BinaryOp::Add => {
            let left = to_primitive_with_env(left, env)?;
            let right = to_primitive_with_env(right, env)?;
            if matches!(left, Value::String(_)) || matches!(right, Value::String(_)) {
                // Reuse the left operand's allocation rather than building a
                // fresh `len(left) + len(right)` buffer each time. This turns a
                // `s += chunk` accumulation loop from O(n^2) copying into the
                // amortized O(n) growth of a single owned `String`, because
                // `to_js_string_with_env` returns a `Value::String`'s backing
                // buffer by move (no copy).
                let mut accumulator = to_js_string_with_env(left, env)?;
                accumulator.push_str(&to_js_string_with_env(right, env)?);
                return Ok(Value::String(accumulator.into()));
            }
            if matches!(left, Value::BigInt(_)) || matches!(right, Value::BigInt(_)) {
                return eval_bigint_binary(left, op, right);
            }
            return Ok(Value::Number(
                to_number_with_env(left, env)? + to_number_with_env(right, env)?,
            ));
        }
        BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
            return eval_relational(left, op, right, env);
        }
        _ => {}
    }

    let left = to_numeric_with_env(left, env)?;
    let right = to_numeric_with_env(right, env)?;

    if matches!(left, Value::BigInt(_)) || matches!(right, Value::BigInt(_)) {
        return eval_bigint_binary(left, op, right);
    }

    let (Value::Number(left), Value::Number(right)) = (left, right) else {
        unreachable!("ToNumeric should return either Number or BigInt")
    };

    let value = match op {
        BinaryOp::Add => left + right,
        BinaryOp::Sub => left - right,
        BinaryOp::Mul => left * right,
        BinaryOp::Pow => number_exponentiate(left, right),
        BinaryOp::Div => left / right,
        BinaryOp::Rem => left % right,
        BinaryOp::Shl => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) << (to_uint32_number(right) & 0x1f),
            )));
        }
        BinaryOp::Shr => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) >> (to_uint32_number(right) & 0x1f),
            )));
        }
        BinaryOp::UShr => {
            return Ok(Value::Number(f64::from(
                to_uint32_number(left) >> (to_uint32_number(right) & 0x1f),
            )));
        }
        BinaryOp::BitwiseAnd => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) & to_int32_number(right),
            )));
        }
        BinaryOp::BitwiseXor => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) ^ to_int32_number(right),
            )));
        }
        BinaryOp::BitwiseOr => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) | to_int32_number(right),
            )));
        }
        BinaryOp::Eq
        | BinaryOp::StrictEq
        | BinaryOp::Ne
        | BinaryOp::StrictNe
        | BinaryOp::Lt
        | BinaryOp::Le
        | BinaryOp::Gt
        | BinaryOp::Ge
        | BinaryOp::In
        | BinaryOp::Instanceof
        | BinaryOp::LogicalAnd
        | BinaryOp::LogicalOr
        | BinaryOp::NullishCoalescing => unreachable!("handled before numeric binary evaluation"),
    };
    Ok(Value::Number(value))
}

/// Number::exponentiate (the `**` operator and `Math.pow`). A NaN exponent, or
/// a base whose magnitude is exactly 1 raised to an infinite exponent, both
/// yield NaN; every other case follows IEEE 754 `pow`. The two NaN cases differ
/// from Rust's `f64::powf`, which returns 1 for `1.powf(±∞)` and `1.powf(NaN)`.
pub(crate) fn number_exponentiate(base: f64, exponent: f64) -> f64 {
    if exponent.is_nan() {
        return f64::NAN;
    }
    if base.abs() == 1.0 && exponent.is_infinite() {
        return f64::NAN;
    }
    base.powf(exponent)
}

fn to_numeric_with_env(value: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    let primitive = match value {
        Value::Object(_)
        | Value::Function(_)
        | Value::Array(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Proxy(_) => to_primitive_with_hint(value, PreferredType::Number, env)?,
        value => value,
    };
    match primitive {
        Value::BigInt(_) => Ok(primitive),
        value => Ok(Value::Number(to_number_with_env(value, env)?)),
    }
}

fn eval_bigint_binary(left: Value, op: BinaryOp, right: Value) -> Result<Value, RuntimeError> {
    let (Value::BigInt(left), Value::BigInt(right)) = (left, right) else {
        return Err(bigint_mix_error());
    };
    let value = match op {
        BinaryOp::Add => left + right,
        BinaryOp::Sub => left - right,
        BinaryOp::Mul => left * right,
        BinaryOp::Div => {
            if right.is_zero() {
                return Err(bigint_division_by_zero_error());
            }
            left / right
        }
        BinaryOp::Rem => {
            if right.is_zero() {
                return Err(bigint_division_by_zero_error());
            }
            left % right
        }
        BinaryOp::Pow => {
            let Some(exponent) = right.to_u32() else {
                return Err(RuntimeError {
                    thrown: None,
                    message: "RangeError: BigInt exponent is too large".to_owned(),
                });
            };
            if right < BigInt::zero() {
                return Err(RuntimeError {
                    thrown: None,
                    message: "RangeError: BigInt exponent must be positive".to_owned(),
                });
            }
            left.pow(exponent)
        }
        BinaryOp::BitwiseAnd => left & right,
        BinaryOp::BitwiseXor => left ^ right,
        BinaryOp::BitwiseOr => left | right,
        BinaryOp::Shl => bigint_left_shift(left, right)?,
        BinaryOp::Shr => bigint_signed_right_shift(left, right)?,
        BinaryOp::UShr => {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: BigInts have no unsigned right shift".to_owned(),
            });
        }
        _ => unreachable!("BigInt binary operator should be arithmetic or bitwise"),
    };
    Ok(Value::BigInt(value))
}

fn bigint_left_shift(left: BigInt, right: BigInt) -> Result<BigInt, RuntimeError> {
    let (negative, amount) = bigint_shift_count(right)?;
    Ok(if negative {
        left >> amount
    } else {
        left << amount
    })
}

fn bigint_signed_right_shift(left: BigInt, right: BigInt) -> Result<BigInt, RuntimeError> {
    let (negative, amount) = bigint_shift_count(right)?;
    Ok(if negative {
        left << amount
    } else {
        left >> amount
    })
}

fn bigint_shift_count(value: BigInt) -> Result<(bool, usize), RuntimeError> {
    let negative = value.sign() == Sign::Minus;
    let magnitude = if negative { -value } else { value };
    let Some(amount) = magnitude.to_usize() else {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: BigInt shift count is too large".to_owned(),
        });
    };
    Ok((negative, amount))
}

fn eval_relational(
    left: Value,
    op: BinaryOp,
    right: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let left = to_primitive_with_env(left, env)?;
    let right = to_primitive_with_env(right, env)?;
    if let (Value::String(left), Value::String(right)) = (&left, &right) {
        let ordering = compare_utf16_code_units(left, right);
        let value = match op {
            BinaryOp::Lt => ordering == Ordering::Less,
            BinaryOp::Le => ordering != Ordering::Greater,
            BinaryOp::Gt => ordering == Ordering::Greater,
            BinaryOp::Ge => ordering != Ordering::Less,
            _ => unreachable!("relational operator required"),
        };
        return Ok(Value::Boolean(value));
    }
    if let (Value::BigInt(left), Value::BigInt(right)) = (&left, &right) {
        let value = match op {
            BinaryOp::Lt => left < right,
            BinaryOp::Le => left <= right,
            BinaryOp::Gt => left > right,
            BinaryOp::Ge => left >= right,
            _ => unreachable!("relational operator required"),
        };
        return Ok(Value::Boolean(value));
    }
    if matches!(left, Value::BigInt(_)) || matches!(right, Value::BigInt(_)) {
        return eval_bigint_mixed_relational(left, op, right, env);
    }

    let left = to_number_with_env(left, env)?;
    let right = to_number_with_env(right, env)?;
    let value = match op {
        BinaryOp::Lt => left < right,
        BinaryOp::Le => left <= right,
        BinaryOp::Gt => left > right,
        BinaryOp::Ge => left >= right,
        _ => unreachable!("relational operator required"),
    };
    Ok(Value::Boolean(value))
}

fn compare_utf16_code_units(left: &str, right: &str) -> Ordering {
    string::string_code_units(left).cmp(&string::string_code_units(right))
}

fn strict_eq(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::String(left), Value::String(right)) => string::string_utf16_eq(left, right),
        _ => left == right,
    }
}

fn abstract_eq(left: &Value, right: &Value, env: &mut CallEnv) -> Result<bool, RuntimeError> {
    match (left, right) {
        (Value::Null, Value::Undefined) | (Value::Undefined, Value::Null) => Ok(true),
        (left, Value::Null | Value::Undefined) if crate::html_dda::is_html_dda(left) => Ok(true),
        (Value::Null | Value::Undefined, right) if crate::html_dda::is_html_dda(right) => Ok(true),
        (Value::BigInt(left), Value::String(right))
        | (Value::String(right), Value::BigInt(left)) => {
            Ok(crate::bigint::parse_bigint_string_value(right).is_some_and(|value| &value == left))
        }
        (Value::BigInt(left), Value::Number(right))
        | (Value::Number(right), Value::BigInt(left)) => Ok(number_bigint_eq(*right, left)),
        (Value::Number(_), Value::String(value)) => {
            Ok(left == &Value::Number(to_number_with_env(Value::String(value.clone()), env)?))
        }
        (Value::String(value), Value::Number(_)) => {
            Ok(&Value::Number(to_number_with_env(Value::String(value.clone()), env)?) == right)
        }
        (Value::Boolean(value), _) => {
            abstract_eq(&Value::Number(if *value { 1.0 } else { 0.0 }), right, env)
        }
        (_, Value::Boolean(value)) => {
            abstract_eq(left, &Value::Number(if *value { 1.0 } else { 0.0 }), env)
        }
        (left, right)
            if is_abstract_eq_object(left) && is_abstract_eq_primitive_for_object(right) =>
        {
            let primitive = to_primitive_with_env(left.clone(), env)?;
            abstract_eq(&primitive, right, env)
        }
        (left, right)
            if is_abstract_eq_primitive_for_object(left) && is_abstract_eq_object(right) =>
        {
            let primitive = to_primitive_with_env(right.clone(), env)?;
            abstract_eq(left, &primitive, env)
        }
        _ => Ok(strict_eq(left, right)),
    }
}

fn is_abstract_eq_primitive_for_object(value: &Value) -> bool {
    matches!(
        value,
        Value::String(_) | Value::Number(_) | Value::BigInt(_)
    ) || is_symbol_primitive_value(value)
}

fn is_abstract_eq_object(value: &Value) -> bool {
    match value {
        Value::Object(object) => !symbol::is_symbol_primitive(object),
        Value::Function(_) | Value::Array(_) | Value::Map(_) | Value::Set(_) | Value::Proxy(_) => {
            true
        }
        _ => false,
    }
}

fn is_symbol_primitive_value(value: &Value) -> bool {
    matches!(value, Value::Object(object) if symbol::is_symbol_primitive(object))
}

fn eval_bigint_mixed_relational(
    left: Value,
    op: BinaryOp,
    right: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let left = bigint_relational_operand(left, env)?;
    let right = bigint_relational_operand(right, env)?;
    Ok(Value::Boolean(match op {
        BinaryOp::Lt => left.partial_cmp(&right) == Some(Ordering::Less),
        BinaryOp::Le => left
            .partial_cmp(&right)
            .is_some_and(|ordering| ordering != Ordering::Greater),
        BinaryOp::Gt => left.partial_cmp(&right) == Some(Ordering::Greater),
        BinaryOp::Ge => left
            .partial_cmp(&right)
            .is_some_and(|ordering| ordering != Ordering::Less),
        _ => unreachable!("relational operator required"),
    }))
}

fn bigint_relational_operand(
    value: Value,
    env: &mut CallEnv,
) -> Result<BigIntComparable, RuntimeError> {
    Ok(match value {
        Value::BigInt(value) => BigIntComparable::BigInt(value),
        // A String operand uses StringToBigInt: an integer literal compares
        // exactly as a BigInt (no f64 precision loss); any other string is
        // undefined, modelled here by NaN so every comparison yields false.
        Value::String(value) => match crate::bigint::parse_bigint_string_value(value.trim()) {
            Some(parsed) => BigIntComparable::BigInt(parsed),
            None => BigIntComparable::Number(f64::NAN),
        },
        value => BigIntComparable::Number(to_number_with_env(value, env)?),
    })
}

enum BigIntComparable {
    BigInt(BigInt),
    Number(f64),
}

impl BigIntComparable {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::BigInt(left), Self::BigInt(right)) => Some(left.cmp(right)),
            (Self::Number(left), Self::Number(right)) => left.partial_cmp(right),
            (Self::Number(number), Self::BigInt(bigint)) => number_bigint_ordering(*number, bigint),
            (Self::BigInt(bigint), Self::Number(number)) => {
                number_bigint_ordering(*number, bigint).map(Ordering::reverse)
            }
        }
    }
}

fn number_bigint_eq(number: f64, bigint: &BigInt) -> bool {
    if !number.is_finite() {
        return false;
    }
    finite_number_to_integer_bigint(number).is_some_and(|integer| integer == *bigint)
}

fn number_bigint_ordering(number: f64, bigint: &BigInt) -> Option<Ordering> {
    if number.is_nan() {
        return None;
    }
    if number == f64::INFINITY {
        return Some(Ordering::Greater);
    }
    if number == f64::NEG_INFINITY {
        return Some(Ordering::Less);
    }
    if let Some(integer) = finite_number_to_integer_bigint(number) {
        return Some(integer.cmp(bigint));
    }
    if let Some(bigint_number) = bigint.to_f64() {
        return number.partial_cmp(&bigint_number);
    }
    Some(match bigint.sign() {
        Sign::Minus => Ordering::Greater,
        Sign::NoSign | Sign::Plus => Ordering::Less,
    })
}

fn finite_number_to_integer_bigint(number: f64) -> Option<BigInt> {
    if !number.is_finite() {
        return None;
    }
    if number == 0.0 {
        return Some(BigInt::zero());
    }

    let bits = number.to_bits();
    let is_negative = bits >> 63 != 0;
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    let (mantissa, exponent) = if exponent_bits == 0 {
        (fraction, -1074)
    } else {
        ((1u64 << 52) | fraction, exponent_bits - 1075)
    };

    let mut integer = if exponent >= 0 {
        BigInt::from(mantissa) << (exponent as usize)
    } else {
        let shift = (-exponent) as u32;
        if shift >= u64::BITS {
            return None;
        }
        let mask = (1u64 << shift) - 1;
        if mantissa & mask != 0 {
            return None;
        }
        BigInt::from(mantissa >> shift)
    };
    if is_negative {
        integer = -integer;
    }
    Some(integer)
}

fn bigint_mix_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: cannot mix BigInt and other types".to_owned(),
    }
}

fn bigint_division_by_zero_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "RangeError: BigInt division by zero".to_owned(),
    }
}

fn eval_instanceof(left: Value, right: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    if matches!(
        &right,
        Value::Function(function) if function.native.is_some_and(error::is_native_error)
    ) {
        return ordinary_has_instance(left, right, env).map(Value::Boolean);
    }

    if let Some(symbol) = symbol::has_instance_symbol(env) {
        let method = crate::property_value_key(right.clone(), &PropertyKey::Symbol(symbol), env)?;
        if !matches!(method, Value::Undefined | Value::Null) {
            let Value::Function(_) = method else {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: Symbol.hasInstance method is not callable".to_owned(),
                });
            };
            let result = call_function(method, right, vec![left], env, false)?;
            return Ok(Value::Boolean(is_truthy(&result)));
        }
    }

    ordinary_has_instance(left, right, env).map(Value::Boolean)
}

pub(crate) fn ordinary_has_instance(
    left: Value,
    right: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let Value::Function(constructor) = right else {
        return Err(RuntimeError {
            thrown: None,
            message: "right-hand side of instanceof is not callable".to_owned(),
        });
    };
    if let Some(bound) = &constructor.bound {
        return ordinary_has_instance(left, bound.target.clone(), env);
    }
    // OrdinaryHasInstance step 3: a non-object operand is never an instance
    // (checked before reading the constructor's `prototype`).
    if !instanceof_prototype_is_object(&left) {
        return Ok(false);
    }
    let prototype = property_value(Value::Function(constructor), "prototype", env)?;
    if !instanceof_prototype_is_object(&prototype) {
        return Err(RuntimeError {
            thrown: None,
            message: "function prototype is not an object".to_owned(),
        });
    }
    // Walk the operand's [[GetPrototypeOf]] chain so a Proxy's getPrototypeOf
    // trap participates (and its invariant checks can throw), instead of a
    // structural walk over the raw prototype slots.
    let mut current = left;
    loop {
        match value_get_prototype_of(current, env)? {
            Value::Null => return Ok(false),
            proto => {
                if proto.same_value(&prototype) {
                    return Ok(true);
                }
                current = proto;
            }
        }
    }
}

/// A value's `[[GetPrototypeOf]]` as a JavaScript value (object or null),
/// dispatching a Proxy's `getPrototypeOf` trap.
fn value_get_prototype_of(value: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    match value {
        Value::Proxy(proxy) => crate::proxy::proxy_get_prototype_of(proxy, env),
        other => Ok(value_prototype_slot(other, env)
            .map(|prototype| prototype.to_value())
            .unwrap_or(Value::Null)),
    }
}

fn instanceof_prototype_is_object(value: &Value) -> bool {
    match value {
        // Symbol primitives are represented as `Value::Object` internally but
        // are not objects for the purposes of OrdinaryHasInstance.
        Value::Object(object) => !crate::symbol::is_symbol_primitive(object),
        Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) | Value::Proxy(_) => {
            true
        }
        _ => false,
    }
}

fn eval_in(left: Value, right: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    // `in` requires an object right operand (checked before HasProperty), and
    // must propagate an abrupt completion from a Proxy `has` trap verbatim
    // rather than masking it as a "not an object" error.
    if !matches!(
        right,
        Value::Object(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Proxy(_)
            | Value::Array(_)
            | Value::Function(_)
    ) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: right operand of in is not an object".to_owned(),
        });
    }
    let key = to_property_key_value(left, env)?;
    has_property_key(right, env, &key).map(Value::Boolean)
}
