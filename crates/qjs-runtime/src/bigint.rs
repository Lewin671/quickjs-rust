use std::collections::HashMap;

use num_bigint::BigInt;
use num_traits::{One, Signed, Zero};

use crate::CallEnv;
use crate::{
    Function, NativeFunction, ObjectRef, PreferredType, Property, RuntimeError, Value,
    inherited_object_prototype_property, symbol, to_js_string_with_env, to_number_with_env,
    to_primitive_with_hint,
};

pub(crate) const BIGINT_DATA_PROPERTY: &str = "\0BigIntData";

pub(crate) fn install_bigint(env: &mut CallEnv, global_this: &Value, object_prototype: ObjectRef) {
    let bigint_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    let bigint_function = Function::new_native(Some("BigInt"), 1, NativeFunction::BigInt, true);
    bigint_prototype.set_to_string_tag("BigInt");
    bigint_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(bigint_function.clone()),
    );
    define_prototype_native(
        &bigint_prototype,
        "toLocaleString",
        0,
        NativeFunction::BigIntPrototypeToLocaleString,
    );
    define_prototype_native(
        &bigint_prototype,
        "toString",
        0,
        NativeFunction::BigIntPrototypeToString,
    );
    define_prototype_native(
        &bigint_prototype,
        "valueOf",
        0,
        NativeFunction::BigIntPrototypeValueOf,
    );
    define_static_native(&bigint_function, "asIntN", 2, NativeFunction::BigIntAsIntN);
    define_static_native(
        &bigint_function,
        "asUintN",
        2,
        NativeFunction::BigIntAsUintN,
    );
    bigint_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(bigint_prototype)),
    );

    let bigint_value = Value::Function(bigint_function);
    env.insert_realm("BigInt".to_owned(), bigint_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("BigInt".to_owned(), bigint_value);
    }
}

pub(crate) fn install_bigint_well_known_symbols(env: &CallEnv) {
    if let Some(prototype) = bigint_prototype(env) {
        symbol::define_well_known_to_string_tag(env, &prototype, "BigInt");
    }
}

fn define_prototype_native(
    prototype: &ObjectRef,
    key: &str,
    length: usize,
    native: NativeFunction,
) {
    prototype.define_non_enumerable(
        key.to_owned(),
        Value::Function(Function::new_native(Some(key), length, native, false)),
    );
}

fn define_static_native(function: &Function, key: &str, length: usize, native: NativeFunction) {
    function.properties.borrow_mut().insert(
        key.to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some(key),
            length,
            native,
            false,
        ))),
    );
}

pub(crate) fn native_bigint(
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: BigInt is not a constructor".to_owned(),
        });
    }
    let value = to_bigint_constructor_value(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    Ok(Value::BigInt(value))
}

pub(crate) fn native_bigint_as_int_n(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let bits = to_index(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let value = to_bigint(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if bits == 0 {
        return Ok(Value::BigInt(BigInt::zero()));
    }
    let modulus = BigInt::one() << bits;
    let unsigned = modulo_bigint(value, &modulus);
    let sign_boundary = BigInt::one() << (bits - 1);
    let signed = if unsigned >= sign_boundary {
        unsigned - modulus
    } else {
        unsigned
    };
    Ok(Value::BigInt(signed))
}

pub(crate) fn native_bigint_as_uint_n(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let bits = to_index(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let value = to_bigint(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if bits == 0 {
        return Ok(Value::BigInt(BigInt::zero()));
    }
    let modulus = BigInt::one() << bits;
    Ok(Value::BigInt(modulo_bigint(value, &modulus)))
}

pub(crate) fn native_bigint_prototype_to_string(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = this_bigint_value(this_value)?;
    let radix = match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => 10,
        value => {
            let radix = to_number_with_env(value, env)?;
            if !radix.is_finite() || radix.fract() != 0.0 || !(2.0..=36.0).contains(&radix) {
                return Err(RuntimeError {
                    thrown: None,
                    message: "RangeError: BigInt radix must be between 2 and 36".to_owned(),
                });
            }
            radix as u32
        }
    };
    Ok(Value::String(bigint_to_string_radix(&value, radix).into()))
}

pub(crate) fn native_bigint_prototype_value_of(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::BigInt(this_bigint_value(this_value)?))
}

pub(crate) fn parse_bigint_literal(raw: &str) -> Result<BigInt, RuntimeError> {
    parse_bigint_text(raw, true).ok_or_else(|| RuntimeError {
        thrown: None,
        message: format!("invalid BigInt literal `{raw}`"),
    })
}

pub(crate) fn parse_bigint_string_value(raw: &str) -> Option<BigInt> {
    parse_bigint_text(raw, false)
}

pub(crate) fn is_bigint_object(object: &ObjectRef) -> bool {
    matches!(
        object.own_property(BIGINT_DATA_PROPERTY),
        Some(Property {
            value: Value::BigInt(_),
            ..
        })
    )
}

pub(crate) fn inherited_bigint_prototype_property(env: &CallEnv, key: &str) -> Option<Value> {
    bigint_prototype(env)
        .and_then(|prototype| prototype.get(key))
        .or_else(|| inherited_object_prototype_property(env, key))
}

pub(crate) fn to_bigint(value: Value, env: &mut CallEnv) -> Result<BigInt, RuntimeError> {
    let primitive = to_primitive_with_hint(value, PreferredType::Number, env)?;
    match primitive {
        Value::BigInt(value) => Ok(value),
        Value::Boolean(value) => Ok(BigInt::from(if value { 1 } else { 0 })),
        Value::Number(_) => Err(invalid_bigint_conversion()),
        Value::String(value) => {
            parse_bigint_string_value(value.trim()).ok_or_else(invalid_bigint_string)
        }
        Value::Null | Value::Undefined => Err(invalid_bigint_conversion()),
        value => parse_bigint_string_value(&to_js_string_with_env(value, env)?)
            .ok_or_else(invalid_bigint_string),
    }
}

fn to_bigint_constructor_value(value: Value, env: &mut CallEnv) -> Result<BigInt, RuntimeError> {
    let primitive = to_primitive_with_hint(value, PreferredType::Number, env)?;
    match primitive {
        Value::Number(number) if number.is_finite() && number.fract() == 0.0 => {
            let text = format!("{number:.0}");
            parse_bigint_string_value(&text).ok_or_else(invalid_bigint_conversion)
        }
        Value::Number(_) => Err(RuntimeError {
            thrown: None,
            message: "RangeError: cannot convert non-integer number to BigInt".to_owned(),
        }),
        value => to_bigint(value, env),
    }
}

fn bigint_prototype(env: &CallEnv) -> Option<ObjectRef> {
    let Some(Value::Function(bigint_function)) = env.get("BigInt") else {
        return None;
    };
    crate::function_prototype(&bigint_function)
}

fn parse_bigint_text(raw: &str, allow_separators: bool) -> Option<BigInt> {
    let (sign, unsigned) = raw.strip_prefix('-').map_or((1, raw), |rest| (-1, rest));
    let unsigned = unsigned.strip_prefix('+').unwrap_or(unsigned);
    let (radix, digits, prefixed) = if let Some(digits) = unsigned
        .strip_prefix("0x")
        .or_else(|| unsigned.strip_prefix("0X"))
    {
        (16, digits, true)
    } else if let Some(digits) = unsigned
        .strip_prefix("0b")
        .or_else(|| unsigned.strip_prefix("0B"))
    {
        (2, digits, true)
    } else if let Some(digits) = unsigned
        .strip_prefix("0o")
        .or_else(|| unsigned.strip_prefix("0O"))
    {
        (8, digits, true)
    } else {
        (10, unsigned, false)
    };
    if digits.is_empty() {
        return (!prefixed).then(BigInt::zero);
    }
    if sign < 0 && radix != 10 {
        return None;
    }
    if digits.contains('_') && !allow_separators {
        return None;
    }
    let digits = if allow_separators {
        digits.replace('_', "")
    } else {
        digits.to_owned()
    };
    let value = BigInt::parse_bytes(digits.as_bytes(), radix)?;
    Some(if sign < 0 { -value } else { value })
}

fn this_bigint_value(value: Value) -> Result<BigInt, RuntimeError> {
    match value {
        Value::BigInt(value) => Ok(value),
        Value::Object(object) => match object.own_property(BIGINT_DATA_PROPERTY) {
            Some(Property {
                value: Value::BigInt(value),
                ..
            }) => Ok(value),
            _ => Err(bigint_method_error("object")),
        },
        _ => Err(bigint_method_error("value")),
    }
}

fn to_index(value: Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
    let number = to_number_with_env(value, env)?;
    if number.is_nan() || number == 0.0 {
        return Ok(0);
    }
    if !number.is_finite() {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: BigInt bit width must be a non-negative integer".to_owned(),
        });
    }
    let integer = number.trunc();
    if integer < 0.0 {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: BigInt bit width must be a non-negative integer".to_owned(),
        });
    }
    if integer > 9_007_199_254_740_991.0 || integer > usize::MAX as f64 {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: BigInt bit width is too large".to_owned(),
        });
    }
    Ok(integer as usize)
}

fn bigint_to_string_radix(value: &BigInt, radix: u32) -> String {
    if value.is_negative() {
        format!("-{}", (-value).to_str_radix(radix))
    } else {
        value.to_str_radix(radix)
    }
}

fn modulo_bigint(value: BigInt, modulus: &BigInt) -> BigInt {
    let remainder = value % modulus;
    if remainder.is_negative() {
        remainder + modulus
    } else {
        remainder
    }
}

fn bigint_method_error(kind: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("TypeError: BigInt.prototype method called on non-BigInt {kind}"),
    }
}

fn invalid_bigint_conversion() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: cannot convert value to BigInt".to_owned(),
    }
}

fn invalid_bigint_string() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "SyntaxError: cannot convert value to BigInt".to_owned(),
    }
}
