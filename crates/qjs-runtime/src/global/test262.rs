//! Native Test262 assertion and data-building helpers.
//!
//! All observable operations use the ordinary runtime property/coercion APIs;
//! helpers report harness failures as structured errors rather than panics.

use crate::{
    ArrayRef, CallEnv, ObjectRef, Property, PropertyKey, RuntimeError, Value,
    bytecode::set_object_property, function_delete_own_property, function_own_property_descriptor,
    object::define_property_on_value_key, property_value, property_value_key,
    string::push_code_point, to_length_with_env, to_number_with_env,
};

pub(crate) fn native_test262_assert_same_value(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let actual = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let expected = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if actual.same_value(&expected) {
        return Ok(Value::Undefined);
    }
    let message = match argument_values.get(2) {
        Some(Value::String(message)) if !message.is_empty() => {
            format!("{message} Expected SameValue to be true")
        }
        _ => "Expected SameValue to be true".to_owned(),
    };
    Err(RuntimeError {
        thrown: None,
        message,
    })
}

pub(crate) fn native_test262_verify_property(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if argument_values.len() < 3 {
        return verify_property_failure(
            "verifyProperty should receive at least 3 arguments: obj, name, and descriptor",
        );
    }
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let key = match argument_values.get(1) {
        Some(Value::String(key)) => key.to_string(),
        _ => return Ok(Value::Boolean(false)),
    };
    if !matches!(
        target,
        Value::Object(_) | Value::Array(_) | Value::Function(_)
    ) {
        return Ok(Value::Boolean(false));
    }
    if matches!(&target, Value::Object(object) if crate::typed_array::is_typed_array_object(object))
    {
        return Ok(Value::Boolean(false));
    }
    if let Some(options) = argument_values.get(3)
        && !matches!(options, Value::Undefined | Value::Null)
    {
        return Ok(Value::Boolean(false));
    }

    let original = crate::object::own_property_descriptor_key(
        target.clone(),
        &PropertyKey::String(key.clone()),
        env,
    )?;
    let desc = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    if matches!(desc, Value::Undefined) {
        if original.is_none() {
            return Ok(Value::Boolean(true));
        }
        return verify_property_failure(&format!("{key} descriptor should be undefined"));
    }
    let Some(original) = original else {
        return verify_property_failure(&format!("{key} should be an own property"));
    };
    if original.is_accessor() {
        return Ok(Value::Boolean(false));
    }
    let Value::Object(desc_object) = desc else {
        if matches!(desc, Value::Array(_) | Value::Function(_) | Value::Proxy(_)) {
            return Ok(Value::Boolean(false));
        }
        return verify_property_failure("The desc argument should be an object or undefined");
    };
    if crate::typed_array::is_typed_array_object(&desc_object) {
        return Ok(Value::Boolean(false));
    }
    for name in desc_object.own_property_names() {
        let Some(desc_field) = desc_object.own_property(&name) else {
            return Ok(Value::Boolean(false));
        };
        if desc_field.is_accessor() {
            return Ok(Value::Boolean(false));
        }
        match name.as_str() {
            "value" | "writable" | "enumerable" | "configurable" => {}
            "get" | "set" => return Ok(Value::Boolean(false)),
            _ => return verify_property_failure(&format!("Invalid descriptor field: {name}")),
        }
    }

    if let Some(expected) = desc_object
        .own_property("value")
        .map(|property| property.value)
    {
        if !expected.same_value(&original.value) {
            return verify_property_failure(&format!("{key} descriptor value mismatch"));
        }
        let actual = property_value_key(target.clone(), &PropertyKey::String(key.clone()), env)?;
        if !expected.same_value(&actual) {
            return verify_property_failure(&format!("{key} value mismatch"));
        }
    }
    if let Some(expected) = optional_bool_descriptor_field(&desc_object, "enumerable")? {
        if expected != original.enumerable || expected != is_string_key_enumerable(&target, &key) {
            return verify_property_failure(&format!("{key} descriptor enumerable mismatch"));
        }
    }
    if let Some(expected) = optional_bool_descriptor_field(&desc_object, "writable")? {
        if expected != original.writable || expected != is_string_key_writable(&target, &key, env)?
        {
            return verify_property_failure(&format!("{key} descriptor writable mismatch"));
        }
    }
    if let Some(expected) = optional_bool_descriptor_field(&desc_object, "configurable")? {
        if expected != original.configurable
            || expected != is_string_key_configurable(&target, &key, &original, env)?
        {
            return verify_property_failure(&format!("{key} descriptor configurable mismatch"));
        }
    }

    Ok(Value::Boolean(true))
}

pub(crate) fn native_test262_to_numbers(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    use num_traits::ToPrimitive;

    let Some(Value::Object(object)) = argument_values.first() else {
        return Ok(Value::Undefined);
    };
    if !crate::typed_array::is_typed_array_object(object) {
        return Ok(Value::Undefined);
    }
    let length = crate::typed_array::typed_array_length(object);
    let values = crate::typed_array::read_view_elements(object, 0, length)
        .into_iter()
        .map(|value| match value {
            Value::BigInt(big) => Value::Number(big.to_f64().unwrap_or(f64::NAN)),
            value => value,
        })
        .collect();
    Ok(Value::Array(ArrayRef::new(values)))
}

pub(crate) fn native_test262_compare_array(
    argument_values: &[Value],
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    let (Some(Value::Array(actual)), Some(Value::Array(expected))) =
        (argument_values.first(), argument_values.get(1))
    else {
        return Ok(Value::Boolean(false));
    };
    let Some(actual_values) = actual.dense_argument_values(env) else {
        return Ok(Value::Boolean(false));
    };
    let Some(expected_values) = expected.dense_argument_values(env) else {
        return Ok(Value::Boolean(false));
    };
    Ok(Value::Boolean(
        actual_values.len() == expected_values.len()
            && actual_values
                .iter()
                .zip(expected_values.iter())
                .all(|(actual, expected)| actual.same_value(expected)),
    ))
}

pub(crate) fn native_test262_assert_iterator_result(
    argument_values: &[Value],
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    let Some(Value::Object(object)) = argument_values.first() else {
        return Ok(Value::Boolean(false));
    };
    let Some(expected_value) = argument_values.get(1) else {
        return Ok(Value::Boolean(false));
    };
    let Some(Value::Boolean(expected_done)) = argument_values.get(2) else {
        return Ok(Value::Boolean(false));
    };
    if !object_uses_default_object_prototype(object, env) || !object.is_extensible() {
        return Ok(Value::Boolean(false));
    }
    if !object.own_property_symbols().is_empty()
        || object.own_property_names() != ["value".to_owned(), "done".to_owned()]
    {
        return Ok(Value::Boolean(false));
    }
    let Some(value_property) = object.own_property("value") else {
        return Ok(Value::Boolean(false));
    };
    let Some(done_property) = object.own_property("done") else {
        return Ok(Value::Boolean(false));
    };
    if !is_default_enumerable_data_property(&value_property)
        || !is_default_enumerable_data_property(&done_property)
    {
        return Ok(Value::Boolean(false));
    }
    let Value::Boolean(actual_done) = done_property.value else {
        return Ok(Value::Boolean(false));
    };
    Ok(Value::Boolean(
        value_property.value.same_value(expected_value) && actual_done == *expected_done,
    ))
}

pub(crate) fn native_test262_assert_packed_array(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Some(Value::Array(array)) = argument_values.first() else {
        return Ok(Value::Boolean(false));
    };
    if !array.is_extensible()
        || !array.is_length_writable()
        || !array.uses_default_prototype()
        || !array.property_names().is_empty()
        || !array.own_property_symbols().is_empty()
    {
        return Ok(Value::Boolean(false));
    }
    Ok(Value::Boolean(
        (0..array.len()).all(|index| array.has_index(index)),
    ))
}

pub(crate) fn native_test262_assert_null_proto_mutable_object(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Some(Value::Object(object)) = argument_values.first() else {
        return Ok(Value::Boolean(false));
    };
    if object.prototype_slot().is_some()
        || !object.is_extensible()
        || !object.own_property_symbols().is_empty()
    {
        return Ok(Value::Boolean(false));
    }
    Ok(Value::Boolean(object.own_property_names().into_iter().all(
        |key| {
            object
                .own_property(&key)
                .is_some_and(|property| is_default_enumerable_data_property(&property))
        },
    )))
}

fn object_uses_default_object_prototype(object: &ObjectRef, env: &CallEnv) -> bool {
    match (object.prototype_slot(), crate::object_prototype(env)) {
        (Some(crate::Prototype::Object(actual)), Some(expected)) => actual.ptr_eq(&expected),
        _ => false,
    }
}

fn is_default_enumerable_data_property(property: &Property) -> bool {
    !property.is_accessor() && property.writable && property.enumerable && property.configurable
}

fn optional_bool_descriptor_field(
    desc_object: &ObjectRef,
    key: &str,
) -> Result<Option<bool>, RuntimeError> {
    let Some(property) = desc_object.own_property(key) else {
        return Ok(None);
    };
    match property.value {
        Value::Undefined => Ok(None),
        Value::Boolean(value) => Ok(Some(value)),
        _ => verify_property_failure(&format!("{key} descriptor field must be boolean")),
    }
}

fn verify_property_failure<T>(message: &str) -> Result<T, RuntimeError> {
    Err(RuntimeError {
        thrown: None,
        message: message.to_owned(),
    })
}

fn is_string_key_enumerable(target: &Value, key: &str) -> bool {
    match target {
        Value::Object(object) => object
            .own_property(key)
            .is_some_and(|property| property.enumerable),
        Value::Array(elements) => crate::array_own_property_descriptor(elements, key)
            .is_some_and(|property| property.enumerable),
        Value::Function(function) => function_own_property_descriptor(function, key)
            .is_some_and(|property| property.enumerable),
        _ => false,
    }
}

fn is_string_key_writable(
    target: &Value,
    key: &str,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let had_value = crate::object::own_property_descriptor_key(
        target.clone(),
        &PropertyKey::String(key.to_owned()),
        env,
    )?
    .is_some();
    let old_value = property_value_key(target.clone(), &PropertyKey::String(key.to_owned()), env)?;
    let mut new_value = if matches!(target, Value::Array(_)) && key == "length" {
        Value::Number((u32::MAX) as f64)
    } else {
        Value::String("unlikelyValue".to_owned().into())
    };
    if new_value.same_value(&old_value) {
        new_value = Value::String("unlikelyValue2".to_owned().into());
    }
    let _ = set_object_property(target.clone(), key.to_owned(), new_value.clone(), env);
    let write_succeeded =
        property_value_key(target.clone(), &PropertyKey::String(key.to_owned()), env)?
            .same_value(&new_value);
    if write_succeeded {
        if had_value {
            let _ = set_object_property(target.clone(), key.to_owned(), old_value, env)?;
        } else {
            delete_string_key(target, key);
        }
    }
    Ok(write_succeeded)
}

fn is_string_key_configurable(
    target: &Value,
    key: &str,
    original: &Property,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let deleted = delete_string_key(target, key);
    let configurable = crate::object::own_property_descriptor_key(
        target.clone(),
        &PropertyKey::String(key.to_owned()),
        env,
    )?
    .is_none();
    if deleted {
        let _ = define_property_on_value_key(
            target.clone(),
            PropertyKey::String(key.to_owned()),
            original.clone(),
            env,
        )?;
    }
    Ok(configurable)
}

fn delete_string_key(target: &Value, key: &str) -> bool {
    match target {
        Value::Object(object) => object.delete_own_property(key),
        Value::Array(elements) => {
            if let Some(index) = crate::array_index_property_key(key) {
                elements.delete_index(index)
            } else {
                elements.delete_property(key)
            }
        }
        Value::Function(function) => function_delete_own_property(function, key),
        _ => false,
    }
}

pub(crate) fn native_test262_build_string(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let args = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let lone_code_points = property_value(args.clone(), "loneCodePoints", env)?;
    let ranges = property_value(args, "ranges", env)?;
    let mut result = String::new();
    append_code_point_list(&mut result, lone_code_points, env)?;
    let ranges_len = array_like_len(ranges.clone(), env)?;
    for index in 0..ranges_len {
        let range =
            property_value_key(ranges.clone(), &PropertyKey::String(index.to_string()), env)?;
        let start = code_point_from_value(
            property_value_key(range.clone(), &PropertyKey::String("0".to_owned()), env)?,
            env,
        )?;
        let end = code_point_from_value(
            property_value_key(range, &PropertyKey::String("1".to_owned()), env)?,
            env,
        )?;
        for code_point in start..=end {
            push_code_point(&mut result, code_point);
        }
    }
    Ok(Value::String(result.into()))
}

fn append_code_point_list(
    result: &mut String,
    values: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let length = array_like_len(values.clone(), env)?;
    for index in 0..length {
        let value =
            property_value_key(values.clone(), &PropertyKey::String(index.to_string()), env)?;
        let code_point = code_point_from_value(value, env)?;
        push_code_point(result, code_point);
    }
    Ok(())
}

fn array_like_len(value: Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
    match value {
        Value::Array(array) => Ok(array.len()),
        value => to_length_with_env(property_value(value, "length", env)?, env),
    }
}

fn code_point_from_value(value: Value, env: &mut CallEnv) -> Result<u32, RuntimeError> {
    let number = to_number_with_env(value, env)?;
    if !number.is_finite() || number < 0.0 || number > 0x10FFFF as f64 || number.trunc() != number {
        return Err(RuntimeError {
            thrown: None,
            message:
                "RangeError: String.fromCodePoint code point must be an integer in [0, 0x10FFFF]"
                    .to_owned(),
        });
    }
    Ok(number as u32)
}
