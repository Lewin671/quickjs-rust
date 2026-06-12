use std::cmp::Ordering;

use num_bigint::{BigInt, Sign};
use num_traits::{ToPrimitive, Zero};
use qjs_ast::{BinaryOp, UnaryOp};

use crate::CallEnv;
use crate::{
    PropertyKey, RuntimeError, Value, call_function, error, has_property_key, is_truthy,
    property_value, string, symbol, to_int32_number, to_js_string_with_env, to_number_with_env,
    to_primitive_with_env, to_property_key_value, to_uint32_number, value_prototype_slot,
};

pub(crate) fn eval_unary(
    op: UnaryOp,
    argument: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    match op {
        UnaryOp::Not => Ok(Value::Boolean(!is_truthy(&argument))),
        UnaryOp::Plus => Ok(Value::Number(to_number_with_env(argument, env)?)),
        UnaryOp::Minus if matches!(argument, Value::BigInt(_)) => {
            let Value::BigInt(value) = argument else {
                unreachable!("BigInt argument was checked before negation")
            };
            Ok(Value::BigInt(-value))
        }
        UnaryOp::BitwiseNot if matches!(argument, Value::BigInt(_)) => {
            let Value::BigInt(value) = argument else {
                unreachable!("BigInt argument was checked before bitwise not")
            };
            Ok(Value::BigInt(!value))
        }
        UnaryOp::Minus => Ok(Value::Number(-to_number_with_env(argument, env)?)),
        UnaryOp::BitwiseNot => Ok(Value::Number(f64::from(!to_int32_number(
            to_number_with_env(argument, env)?,
        )))),
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
                return Ok(Value::String(accumulator));
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

    let left = to_primitive_with_env(left, env)?;
    let right = to_primitive_with_env(right, env)?;

    if matches!(left, Value::BigInt(_)) || matches!(right, Value::BigInt(_)) {
        return eval_bigint_binary(left, op, right);
    }

    let left = to_number_with_env(left, env)?;
    let right = to_number_with_env(right, env)?;

    let value = match op {
        BinaryOp::Add => left + right,
        BinaryOp::Sub => left - right,
        BinaryOp::Mul => left * right,
        BinaryOp::Pow => left.powf(right),
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
        BinaryOp::Shl | BinaryOp::Shr | BinaryOp::UShr => {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: BigInt shifts are not supported yet".to_owned(),
            });
        }
        _ => unreachable!("BigInt binary operator should be arithmetic or bitwise"),
    };
    Ok(Value::BigInt(value))
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
        (
            Value::Object(_) | Value::Function(_) | Value::Array(_),
            Value::String(_) | Value::Number(_) | Value::BigInt(_),
        ) => {
            let primitive = to_primitive_with_env(left.clone(), env)?;
            abstract_eq(&primitive, right, env)
        }
        (
            Value::String(_) | Value::Number(_) | Value::BigInt(_),
            Value::Object(_) | Value::Function(_) | Value::Array(_),
        ) => {
            let primitive = to_primitive_with_env(right.clone(), env)?;
            abstract_eq(left, &primitive, env)
        }
        _ => Ok(strict_eq(left, right)),
    }
}

fn eval_bigint_mixed_relational(
    left: Value,
    op: BinaryOp,
    right: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let left = match left {
        Value::BigInt(value) => BigIntComparable::BigInt(value),
        value => BigIntComparable::Number(to_number_with_env(value, env)?),
    };
    let right = match right {
        Value::BigInt(value) => BigIntComparable::BigInt(value),
        value => BigIntComparable::Number(to_number_with_env(value, env)?),
    };
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
    number_bigint_ordering(number, bigint) == Some(Ordering::Equal)
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
    if let Some(bigint_number) = bigint.to_f64() {
        return number.partial_cmp(&bigint_number);
    }
    Some(match bigint.sign() {
        Sign::Minus => Ordering::Greater,
        Sign::NoSign | Sign::Plus => Ordering::Less,
    })
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
    let Some(left_prototype) = value_prototype_slot(left, env) else {
        return Ok(false);
    };
    let prototype = property_value(Value::Function(constructor), "prototype", env)?;
    if !instanceof_prototype_is_object(&prototype) {
        return Err(RuntimeError {
            thrown: None,
            message: "function prototype is not an object".to_owned(),
        });
    }
    Ok(left_prototype.chain_contains_value(&prototype))
}

fn instanceof_prototype_is_object(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_)
            | Value::Array(_)
            | Value::Function(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Proxy(_)
    )
}

fn eval_in(left: Value, right: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    let key = to_property_key_value(left, env)?;
    has_property_key(right, env, &key)
        .map(Value::Boolean)
        .map_err(|_| RuntimeError {
            thrown: None,
            message: "right operand of in is not an object".to_owned(),
        })
}
