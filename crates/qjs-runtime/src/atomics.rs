use std::collections::HashMap;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::{
    CallEnv, Function, NativeFunction, ObjectRef, RuntimeError, Value, symbol, to_number_with_env,
    typed_array,
};

#[derive(Clone, Copy)]
pub(crate) enum AtomicOp {
    Add,
    And,
    Exchange,
    Or,
    Sub,
    Xor,
}

pub(crate) fn install_atomics(env: &mut CallEnv, global_this: &Value, object_prototype: ObjectRef) {
    let atomics = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    atomics.set_to_string_tag("Atomics");
    symbol::define_well_known_to_string_tag(env, &atomics, "Atomics");
    for (name, length, native) in [
        ("add", 3, NativeFunction::AtomicsAdd),
        ("and", 3, NativeFunction::AtomicsAnd),
        ("compareExchange", 4, NativeFunction::AtomicsCompareExchange),
        ("exchange", 3, NativeFunction::AtomicsExchange),
        ("isLockFree", 1, NativeFunction::AtomicsIsLockFree),
        ("load", 2, NativeFunction::AtomicsLoad),
        ("notify", 3, NativeFunction::AtomicsNotify),
        ("or", 3, NativeFunction::AtomicsOr),
        ("pause", 0, NativeFunction::AtomicsPause),
        ("store", 3, NativeFunction::AtomicsStore),
        ("sub", 3, NativeFunction::AtomicsSub),
        ("wait", 4, NativeFunction::AtomicsWait),
        ("xor", 3, NativeFunction::AtomicsXor),
    ] {
        atomics.define_non_enumerable(
            name.to_owned(),
            Value::Function(Function::new_native(Some(name), length, native, false)),
        );
    }

    let value = Value::Object(atomics);
    env.insert_realm("Atomics".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("Atomics".to_owned(), value);
    }
}

pub(crate) fn native_atomics_is_lock_free(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let size = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    Ok(Value::Boolean(
        size.fract() == 0.0 && matches!(size as usize, 1 | 2 | 4 | 8),
    ))
}

pub(crate) fn native_atomics_load(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let access = validate_atomic_access(argument_values, AccessMode::Read, env)?;
    Ok(typed_array::get_view_element(&access.object, access.index))
}

pub(crate) fn native_atomics_store(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let access = validate_atomic_access(argument_values, AccessMode::Write, env)?;
    let value = to_atomic_store_value(
        access.kind,
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let stored = coerce_atomic_value(access.kind, value.clone(), env)?;
    typed_array::set_view_elements(&access.object, access.index, [stored]);
    Ok(value)
}

pub(crate) fn native_atomics_pause(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    match argument_values.first() {
        None | Some(Value::Undefined) => Ok(Value::Undefined),
        Some(Value::Number(number)) if number.is_finite() && number.fract() == 0.0 => {
            Ok(Value::Undefined)
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: Atomics.pause iterationNumber must be an integral Number"
                .to_owned(),
        }),
    }
}

pub(crate) fn native_atomics_notify(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let (object, length) = typed_array::validate_typed_array(&target)?;
    let kind = typed_array::typed_array_kind(&object);
    if !is_waitable_kind(kind) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Atomics.notify requires an Int32Array or BigInt64Array".to_owned(),
        });
    }
    let index = to_atomic_index(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if index >= length {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: Atomics index is out of range".to_owned(),
        });
    }
    let _count = match argument_values.get(2) {
        None | Some(Value::Undefined) => f64::INFINITY,
        Some(value) => {
            let number = to_number_with_env(value.clone(), env)?;
            if number.is_nan() {
                0.0
            } else {
                number.trunc().max(0.0)
            }
        }
    };
    let Some(buffer) = typed_array::typed_array_buffer(&object) else {
        return Ok(Value::Number(0.0));
    };
    if !crate::array_buffer::is_shared_array_buffer_object(&buffer) {
        return Ok(Value::Number(0.0));
    }
    Ok(Value::Number(0.0))
}

pub(crate) fn native_atomics_wait(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let (object, length) = typed_array::validate_typed_array(&target)?;
    let kind = typed_array::typed_array_kind(&object);
    if !is_waitable_kind(kind) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Atomics.wait requires an Int32Array or BigInt64Array".to_owned(),
        });
    }
    let Some(buffer) = typed_array::typed_array_buffer(&object) else {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Atomics.wait requires a shared buffer".to_owned(),
        });
    };
    if !crate::array_buffer::is_shared_array_buffer_object(&buffer) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Atomics.wait requires a shared buffer".to_owned(),
        });
    }
    let index = to_atomic_index(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if index >= length {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: Atomics index is out of range".to_owned(),
        });
    }
    let value = coerce_atomic_value(
        kind,
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let timeout = match argument_values.get(3) {
        None | Some(Value::Undefined) => f64::INFINITY,
        Some(value) => {
            let number = to_number_with_env(value.clone(), env)?;
            if number.is_nan() {
                f64::INFINITY
            } else {
                number.max(0.0)
            }
        }
    };
    // AgentCanSuspend(): a `CanBlockIsFalse`-flagged case runs in an agent whose
    // `[[CanBlock]]` is false, so `Atomics.wait` must throw a TypeError rather
    // than block. The check follows the spec order: after the typed-array,
    // index, value, and timeout coercions above.
    #[cfg(feature = "agents")]
    if env
        .agent_context()
        .is_some_and(|context| !context.can_block)
    {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Atomics.wait cannot be used in an agent that cannot be suspended"
                .to_owned(),
        });
    }
    // This engine runs a single agent (no worker threads), so no other agent
    // can ever notify a waiter. Per the spec single-agent semantics: load the
    // current value; if it differs from the comparand return "not-equal";
    // otherwise the wait reaches its timeout (we need not actually block, since
    // `timeout` is only an upper bound and no notifier can exist) → "timed-out".
    let _ = timeout;
    let current = typed_array::get_view_element(&object, index);
    let equal = match (&current, &value) {
        (Value::Number(current), Value::Number(value)) => current == value,
        (Value::BigInt(current), Value::BigInt(value)) => current == value,
        _ => false,
    };
    let result = if equal { "timed-out" } else { "not-equal" };
    Ok(Value::String(result.to_owned().into()))
}

pub(crate) fn native_atomics_read_modify_write(
    argument_values: &[Value],
    op: AtomicOp,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let access = validate_atomic_access(argument_values, AccessMode::Write, env)?;
    let value = coerce_atomic_value(
        access.kind,
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let old = typed_array::get_view_element(&access.object, access.index);
    let new_value = apply_atomic_op(access.kind, &old, &value, op);
    typed_array::set_view_elements(&access.object, access.index, [new_value]);
    Ok(old)
}

pub(crate) fn native_atomics_compare_exchange(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let access = validate_atomic_access(argument_values, AccessMode::Write, env)?;
    let expected = coerce_atomic_value(
        access.kind,
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let replacement = coerce_atomic_value(
        access.kind,
        argument_values.get(3).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let old = typed_array::get_view_element(&access.object, access.index);
    if atomic_values_equal(&old, &expected) {
        typed_array::set_view_elements(&access.object, access.index, [replacement]);
    }
    Ok(old)
}

#[derive(Clone, Copy)]
enum AccessMode {
    Read,
    Write,
}

struct AtomicAccess {
    object: ObjectRef,
    kind: NativeFunction,
    index: usize,
}

fn validate_atomic_access(
    argument_values: &[Value],
    access_mode: AccessMode,
    env: &mut CallEnv,
) -> Result<AtomicAccess, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let (object, length) = match access_mode {
        AccessMode::Read => typed_array::validate_typed_array(&target)?,
        AccessMode::Write => typed_array::validate_typed_array_write(&target)?,
    };
    let kind = typed_array::typed_array_kind(&object);
    if !is_atomic_integer_kind(kind) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Atomics operation requires an integer TypedArray".to_owned(),
        });
    }
    let index = to_atomic_index(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if index >= length {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: Atomics index is out of range".to_owned(),
        });
    }
    Ok(AtomicAccess {
        object,
        kind,
        index,
    })
}

fn to_atomic_index(value: Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
    let number = to_number_with_env(value, env)?;
    let integer = if number.is_nan() { 0.0 } else { number.trunc() };
    if integer < 0.0 || !integer.is_finite() || integer > usize::MAX as f64 {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid Atomics index".to_owned(),
        });
    }
    Ok(integer as usize)
}

fn is_atomic_integer_kind(kind: NativeFunction) -> bool {
    matches!(
        kind,
        NativeFunction::Int8Array
            | NativeFunction::Uint8Array
            | NativeFunction::Int16Array
            | NativeFunction::Uint16Array
            | NativeFunction::Int32Array
            | NativeFunction::Uint32Array
            | NativeFunction::BigInt64Array
            | NativeFunction::BigUint64Array
    )
}

fn is_waitable_kind(kind: NativeFunction) -> bool {
    matches!(
        kind,
        NativeFunction::Int32Array | NativeFunction::BigInt64Array
    )
}

fn coerce_atomic_value(
    kind: NativeFunction,
    value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    typed_array::coerce_element(kind, value, env)
}

fn to_atomic_store_value(
    kind: NativeFunction,
    value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if typed_array::is_big_int_kind(kind) {
        return crate::bigint::to_bigint(value, env).map(Value::BigInt);
    }
    let number = to_number_with_env(value, env)?;
    let integer = if number.is_nan() {
        0.0
    } else {
        let integer = number.trunc();
        if integer == 0.0 { 0.0 } else { integer }
    };
    Ok(Value::Number(integer))
}

fn apply_atomic_op(kind: NativeFunction, old: &Value, value: &Value, op: AtomicOp) -> Value {
    if typed_array::is_big_int_kind(kind) {
        return apply_big_int_atomic_op(kind, old, value, op);
    }

    let old = number_to_i64(old);
    let value = number_to_i64(value);
    let result = match op {
        AtomicOp::Add => old.wrapping_add(value),
        AtomicOp::And => old & value,
        AtomicOp::Exchange => value,
        AtomicOp::Or => old | value,
        AtomicOp::Sub => old.wrapping_sub(value),
        AtomicOp::Xor => old ^ value,
    };
    match kind {
        NativeFunction::Uint8Array => Value::Number((result as u8) as f64),
        NativeFunction::Int8Array => Value::Number((result as i8) as f64),
        NativeFunction::Uint16Array => Value::Number((result as u16) as f64),
        NativeFunction::Int16Array => Value::Number((result as i16) as f64),
        NativeFunction::Uint32Array => Value::Number((result as u32) as f64),
        NativeFunction::Int32Array => Value::Number((result as i32) as f64),
        _ => Value::Number(result as f64),
    }
}

fn apply_big_int_atomic_op(
    kind: NativeFunction,
    old: &Value,
    value: &Value,
    op: AtomicOp,
) -> Value {
    let old = big_int_to_i64(old);
    let value = big_int_to_i64(value);
    let result = match op {
        AtomicOp::Add => old.wrapping_add(value),
        AtomicOp::And => old & value,
        AtomicOp::Exchange => value,
        AtomicOp::Or => old | value,
        AtomicOp::Sub => old.wrapping_sub(value),
        AtomicOp::Xor => old ^ value,
    };
    if matches!(kind, NativeFunction::BigUint64Array) {
        Value::BigInt(BigInt::from(result as u64))
    } else {
        Value::BigInt(BigInt::from(result))
    }
}

fn atomic_values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::BigInt(left), Value::BigInt(right)) => left == right,
        (Value::Number(left), Value::Number(right)) => left == right,
        _ => false,
    }
}

fn number_to_i64(value: &Value) -> i64 {
    match value {
        Value::Number(number) => *number as i64,
        _ => 0,
    }
}

fn big_int_to_i64(value: &Value) -> i64 {
    match value {
        Value::BigInt(value) => {
            let modulo = BigInt::from(1u128 << 64);
            let wrapped = ((value % &modulo) + &modulo) % &modulo;
            wrapped.to_u64().map(|value| value as i64).unwrap_or(0)
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests;
