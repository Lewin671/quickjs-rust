use qjs_parser::{EvalParseContext, parse_direct_eval_script, parse_script};
use std::collections::HashSet;

use crate::CallEnv;
use crate::{
    ArrayRef, Function, GLOBAL_THIS_BINDING, NativeFunction, ObjectRef, Property, RuntimeError,
    Value,
    bytecode::{
        compile_direct_eval_script, eval_bytecode_with_env,
        eval_bytecode_with_env_ephemeral_global_lexicals, set_object_property,
    },
    function_delete_own_property, function_own_property_descriptor,
    object::define_property_on_value_key,
    string::{string_code_units, string_from_code_unit, surrogate_escape_code_unit},
    to_js_string_with_env, to_length_with_env, to_number_with_env,
};
use crate::{PropertyKey, property_value, property_value_key};

const DYNAMIC_FUNCTION_REALM_GLOBAL: &str = "__quickjsRustDynamicFunctionRealm";

pub(super) fn install_globals(env: &mut CallEnv, global_this: &Value) {
    env.insert_realm("NaN".to_owned(), Value::Number(f64::NAN));
    env.insert_realm("Infinity".to_owned(), Value::Number(f64::INFINITY));
    env.insert_realm("undefined".to_owned(), Value::Undefined);
    env.insert_realm("globalThis".to_owned(), global_this.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_property(
            "NaN".to_owned(),
            Property::data(Value::Number(f64::NAN), false, false, false),
        );
        global_object.define_property(
            "Infinity".to_owned(),
            Property::data(Value::Number(f64::INFINITY), false, false, false),
        );
        global_object.define_property(
            "undefined".to_owned(),
            Property::data(Value::Undefined, false, false, false),
        );
        global_object.define_property(
            "globalThis".to_owned(),
            Property::data(global_this.clone(), false, true, true),
        );
        global_object.define_property(
            "NaN".to_owned(),
            Property::data(Value::Number(f64::NAN), false, false, false),
        );
        global_object.define_property(
            "Infinity".to_owned(),
            Property::data(Value::Number(f64::INFINITY), false, false, false),
        );
        global_object.define_property(
            "undefined".to_owned(),
            Property::data(Value::Undefined, false, false, false),
        );
    }

    define_global_function(
        env,
        global_this,
        "isFinite",
        1,
        NativeFunction::GlobalIsFinite,
    );
    define_global_function(env, global_this, "isNaN", 1, NativeFunction::GlobalIsNaN);
    define_global_function(env, global_this, "decodeURI", 1, NativeFunction::DecodeUri);
    define_global_function(
        env,
        global_this,
        "decodeURIComponent",
        1,
        NativeFunction::DecodeUriComponent,
    );
    define_global_function(env, global_this, "encodeURI", 1, NativeFunction::EncodeUri);
    define_global_function(
        env,
        global_this,
        "encodeURIComponent",
        1,
        NativeFunction::EncodeUriComponent,
    );
    define_global_function(env, global_this, "eval", 1, NativeFunction::Eval);
    define_global_function(env, global_this, "print", 1, NativeFunction::Print);
    define_global_function(
        env,
        global_this,
        "__quickjsRustAssertSameValue",
        3,
        NativeFunction::Test262AssertSameValue,
    );
    define_global_function(
        env,
        global_this,
        "__quickjsRustVerifyProperty",
        4,
        NativeFunction::Test262VerifyProperty,
    );
    define_global_function(env, global_this, "escape", 1, NativeFunction::Escape);
    define_global_function(env, global_this, "unescape", 1, NativeFunction::Unescape);
    define_is_html_dda(env, global_this);
    define_global_function(
        env,
        global_this,
        "__quickjsRustDetachArrayBuffer",
        1,
        NativeFunction::DetachArrayBuffer,
    );
    define_global_function(
        env,
        global_this,
        "__quickjsRustEvalScript",
        1,
        NativeFunction::EvalScript,
    );
    define_global_function(
        env,
        global_this,
        "__quickjsRustBuildString",
        1,
        NativeFunction::Test262BuildString,
    );
    define_global_function(
        env,
        global_this,
        "__quickjsRustToNumbers",
        1,
        NativeFunction::Test262ToNumbers,
    );
    define_global_function(
        env,
        global_this,
        "__quickjsRustCompareArray",
        2,
        NativeFunction::Test262CompareArray,
    );
    define_global_function(
        env,
        global_this,
        "__quickjsRustAssertIteratorResult",
        3,
        NativeFunction::Test262AssertIteratorResult,
    );
    define_global_function(
        env,
        global_this,
        "__quickjsRustAssertPackedArray",
        1,
        NativeFunction::Test262AssertPackedArray,
    );
    define_global_function(
        env,
        global_this,
        "__quickjsRustAssertNullProtoMutableObject",
        1,
        NativeFunction::Test262AssertNullProtoMutableObject,
    );
}

fn define_global_function(
    env: &mut CallEnv,
    global_this: &Value,
    key: &str,
    length: usize,
    native: NativeFunction,
) {
    let value = Value::Function(Function::new_native(Some(key), length, native, false));
    env.insert_realm(key.to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable(key.to_owned(), value);
    }
}

fn define_is_html_dda(env: &mut CallEnv, global_this: &Value) {
    let key = "__quickjsRustIsHTMLDDA";
    let value = Value::Function(crate::html_dda::new_is_html_dda_function());
    env.insert_realm(key.to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable(key.to_owned(), value);
    }
}

/// Host `print`: stringifies each argument, joins them with spaces, writes the
/// line to stdout, and returns `undefined`. This is a host shim (QuickJS-NG's
/// `qjs` exposes the same global) used, among other things, by the Test262
/// async `$DONE` channel; the runtime stays unaware of Test262 conventions.
pub(super) fn native_global_print(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let mut line = String::new();
    for (index, value) in argument_values.iter().enumerate() {
        if index > 0 {
            line.push(' ');
        }
        line.push_str(&to_js_string_with_env(value.clone(), env)?);
    }
    println!("{line}");
    Ok(Value::Undefined)
}

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
    match (object.prototype(), crate::object_prototype(env)) {
        (Some(actual), Some(expected)) => actual.ptr_eq(&expected),
        _ => false,
    }
}

fn is_default_enumerable_data_property(property: &Property) -> bool {
    !property.accessor && property.writable && property.enumerable && property.configurable
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
        Value::Array(elements) => match key.parse::<usize>() {
            Ok(index) => elements.delete_index(index),
            Err(_) => elements.delete_property(key),
        },
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

fn push_code_point(result: &mut String, code_point: u32) {
    match char::from_u32(code_point) {
        Some(character) => result.push(character),
        None => result.push_str(&string_from_code_unit(code_point as u16)),
    }
}

pub(super) fn native_global_is_finite(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(to_number_with_env(value, env)?.is_finite()))
}

pub(super) fn native_global_is_nan(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(to_number_with_env(value, env)?.is_nan()))
}

pub(super) fn native_global_encode_uri(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    encode_uri_string(&source).map(|s| Value::String(s.into()))
}

pub(super) fn native_global_encode_uri_component(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    encode_uri_component_string(&source).map(|s| Value::String(s.into()))
}

pub(super) fn native_global_decode_uri(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    decode_uri_string(&source).map(|s| Value::String(s.into()))
}

pub(super) fn native_global_decode_uri_component(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    decode_uri_component_string(&source).map(|s| Value::String(s.into()))
}

pub(crate) fn decode_uri_string(source: &str) -> Result<String, RuntimeError> {
    decode_uri(source, UriDecodeKind::Uri)
}

pub(crate) fn decode_uri_component_string(source: &str) -> Result<String, RuntimeError> {
    decode_uri(source, UriDecodeKind::Component)
}

pub(crate) fn encode_uri_string(source: &str) -> Result<String, RuntimeError> {
    encode_uri(source, UriEncodeKind::Uri)
}

pub(crate) fn encode_uri_component_string(source: &str) -> Result<String, RuntimeError> {
    encode_uri(source, UriEncodeKind::Component)
}

pub(super) fn native_global_eval(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let Value::String(source) = value else {
        return Ok(value);
    };
    let direct_eval = matches!(
        env.get(crate::DIRECT_EVAL_BINDING),
        Some(Value::Boolean(true))
    );
    if let Some(value) = try_eval_regexp_literal_source(&source, env)? {
        return Ok(value);
    }
    if eval_source_is_only_comments_and_whitespace(&source) {
        return Ok(Value::Undefined);
    }
    let script = if direct_eval {
        parse_direct_eval_script(&source, direct_eval_parse_context(env))
    } else {
        parse_script(&source)
    }
    .map_err(|error| RuntimeError {
        thrown: None,
        message: format!("SyntaxError: {}", error.message),
    })?;
    let mut eval_env = if direct_eval {
        env.clone()
    } else {
        indirect_eval_frame(env)
    };
    let direct_function_eval = direct_eval && eval_env.get_local("this").is_some();
    let direct_parameter_eval = direct_function_eval
        && matches!(
            eval_env.get(crate::DIRECT_EVAL_IN_PARAMETER_SCOPE_BINDING),
            Some(Value::Boolean(true))
        );
    // Direct eval inside strict code is itself strict even without its own
    // "use strict" prologue; seed the compiler so Annex B block-function
    // hoisting is correctly suppressed. Indirect eval is sloppy unless its own
    // body opts in.
    let caller_strict = direct_eval
        && matches!(
            env.get(crate::DIRECT_EVAL_STRICT_BINDING),
            Some(Value::Boolean(true))
        );
    let mut bytecode = compile_direct_eval_script(&script, caller_strict)?;
    let eval_strict = bytecode.is_strict();
    if direct_function_eval
        && matches!(
            eval_env.get(crate::DIRECT_EVAL_ARGUMENTS_BINDING),
            Some(Value::Boolean(true))
        )
        && matches!(
            eval_env.get(crate::DIRECT_EVAL_IN_PARAMETER_SCOPE_BINDING),
            Some(Value::Boolean(true))
        )
        && bytecode
            .hoisted_local_names()
            .any(|name| name == "arguments")
    {
        // EvalDeclarationInstantiation: a direct eval in a formal-parameter
        // default may not hoist a `var`/`function` named `arguments`. With
        // parameter expressions the parameter list has its own environment that
        // already binds `arguments` (the arguments object, or an `arguments`
        // parameter), so the eval's separate var declaration collides with it
        // and is a SyntaxError. A *body*-scope direct eval shares the function
        // var environment with `arguments` and may redeclare it (`var arguments`
        // in a plain function body is allowed in sloppy code).
        return Err(RuntimeError {
            thrown: None,
            message: "SyntaxError: cannot declare 'arguments' in function eval".to_owned(),
        });
    }
    // A direct eval always gets its own declarative lexical environment, so its
    // `let`/`const`/`class` declarations never clash with an existing *lexical*
    // global binding (`let outside; eval('let outside;')` is two distinct
    // bindings). They must still not clash with a non-configurable global *var*
    // binding. Indirect eval also gets a fresh declarative lexical environment,
    // so its lexicals do not collide with or persist into global lexicals.
    if direct_function_eval {
        // A function-scope direct eval declares no global lexicals at all.
    } else {
        validate_eval_global_lexical_bindings(&bytecode, &eval_env, false, false)?;
    }
    let caller_locals = eval_env.locals().keys().cloned().collect::<HashSet<_>>();
    let hoisted_names = bytecode
        .hoisted_local_names()
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    let hoisted_function_names = bytecode
        .hoisted_function_names()
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    if !direct_function_eval && !eval_strict {
        validate_sloppy_global_eval_declarations(
            &bytecode,
            &eval_env,
            &caller_locals,
            &hoisted_function_names,
            false,
        )?;
    }
    if direct_function_eval && !eval_strict {
        validate_sloppy_function_eval_declarations(&bytecode, &eval_env)?;
    }
    if direct_function_eval && !eval_strict {
        bytecode.mark_eval_deletable_locals(
            hoisted_names
                .iter()
                .filter(|name| !caller_locals.contains(*name))
                .cloned(),
        );
    }
    let mut strict_direct_writeback_env = (direct_eval && eval_strict).then(|| env.clone());
    initialize_direct_eval_bindings(
        &bytecode,
        &mut eval_env,
        direct_function_eval,
        direct_parameter_eval,
        &caller_locals,
        eval_strict,
    );
    let result = if direct_eval {
        eval_bytecode_with_env(&bytecode, eval_env.clone())
    } else {
        eval_bytecode_with_env_ephemeral_global_lexicals(&bytecode, eval_env.clone())
    };
    let writeback_names = hoisted_names
        .iter()
        .cloned()
        .chain(bytecode.written_binding_names())
        .collect::<HashSet<_>>();
    for name in writeback_names {
        let name = name.as_str();
        let binding = if direct_function_eval && hoisted_names.contains(name) {
            result.frame_binding(name).or_else(|| result.binding(name))
        } else {
            result.binding(name)
        };
        if let Some(value) = binding {
            if let Some(writeback_env) = strict_direct_writeback_env.as_mut() {
                if hoisted_names.contains(name) {
                    continue;
                }
                if caller_locals.contains(name) {
                    // Strict direct eval runs declarations in its own eval
                    // variable environment, but ordinary assignments to
                    // caller-scope bindings still write through to the caller.
                    writeback_env.insert(name.to_owned(), value.clone());
                }
                continue;
            }
            if direct_parameter_eval && hoisted_names.contains(name) && !eval_strict {
                eval_env.insert(name.to_owned(), value.clone());
                eval_env.insert(
                    format!(
                        "{}{}",
                        crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX,
                        name
                    ),
                    Value::Boolean(true),
                );
                update_direct_eval_parameter_captured_functions(&mut eval_env, name, value.clone());
            } else if caller_locals.contains(name) {
                // A caller frame binding (an outer `let`/`var` the eval'd code
                // assigned): write it back through the frame so the caller's
                // slot sees the update.
                eval_env.insert(name.to_owned(), value.clone());
            } else if direct_function_eval {
                eval_env.insert(name.to_owned(), value.clone());
                if hoisted_names.contains(name) {
                    update_direct_eval_captured_functions(&mut eval_env, name, value.clone());
                }
            } else if eval_strict && !direct_eval && hoisted_names.contains(name) {
                continue;
            } else if hoisted_function_names.contains(name) {
                create_eval_global_function_binding(&mut eval_env, name, value.clone());
            } else if hoisted_names.contains(name) {
                create_eval_global_var_binding(&mut eval_env, name, value.clone());
            } else {
                define_eval_global_binding(&mut eval_env, name, value.clone());
            }
        }
    }
    // Indirect eval evaluates its lexical declarations (let/const/class) in a
    // fresh declarative environment whose parent is the global environment;
    // those bindings are discarded when the eval completes and never become
    // global lexical bindings. Only var/function declarations (handled above
    // via define_eval_global_binding) reach the global var environment.
    if direct_eval {
        *env = strict_direct_writeback_env.unwrap_or(eval_env);
    }
    result.value
}

pub(crate) fn eval_source_is_only_comments_and_whitespace(source: &str) -> bool {
    let mut index = 0;
    while index < source.len() {
        let Some(ch) = source[index..].chars().next() else {
            break;
        };
        if is_js_whitespace_or_line_terminator(ch) {
            index += ch.len_utf8();
            continue;
        }
        let rest = &source[index..];
        if let Some(line_comment) = rest.strip_prefix("//") {
            return !line_comment.chars().any(is_js_line_terminator);
        }
        if let Some(block_comment) = rest.strip_prefix("/*") {
            let Some(close) = block_comment.find("*/") else {
                return false;
            };
            index += 2 + close + 2;
            continue;
        }
        return false;
    }
    true
}

fn is_js_whitespace_or_line_terminator(ch: char) -> bool {
    matches!(
        ch,
        '\u{0009}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{FEFF}'
            | '\u{1680}'
            | '\u{2000}'
            | '\u{2001}'
            | '\u{2002}'
            | '\u{2003}'
            | '\u{2004}'
            | '\u{2005}'
            | '\u{2006}'
            | '\u{2007}'
            | '\u{2008}'
            | '\u{2009}'
            | '\u{200A}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{000A}'
            | '\u{000D}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

fn is_js_line_terminator(ch: char) -> bool {
    matches!(ch, '\u{000A}' | '\u{000D}' | '\u{2028}' | '\u{2029}')
}

fn indirect_eval_frame(env: &CallEnv) -> CallEnv {
    let mut eval_env = env.indirect_eval_frame();
    let Some(global) = marked_dynamic_realm_global(env) else {
        return eval_env;
    };
    eval_env.insert(
        DYNAMIC_FUNCTION_REALM_GLOBAL.to_owned(),
        Value::Object(global.clone()),
    );
    eval_env.insert(
        GLOBAL_THIS_BINDING.to_owned(),
        Value::Object(global.clone()),
    );
    eval_env.insert("globalThis".to_owned(), Value::Object(global.clone()));
    eval_env.insert("this".to_owned(), Value::Object(global.clone()));
    for name in global.own_property_names() {
        if name.starts_with('\0') {
            continue;
        }
        if let Some(property) = global.own_property(&name) {
            eval_env.insert(name, property.value);
        }
    }
    eval_env
}

fn marked_dynamic_realm_global(env: &CallEnv) -> Option<ObjectRef> {
    env.get(DYNAMIC_FUNCTION_REALM_GLOBAL)
        .and_then(object_value)
        .or_else(|| {
            env.get(GLOBAL_THIS_BINDING)
                .and_then(object_value)
                .and_then(|global_this| {
                    global_this
                        .own_property(DYNAMIC_FUNCTION_REALM_GLOBAL)
                        .and_then(|property| object_value(property.value))
                })
        })
}

fn object_value(value: Value) -> Option<ObjectRef> {
    match value {
        Value::Object(object) => Some(object),
        _ => None,
    }
}

/// Host `$262.evalScript`: evaluates `source` as a global script in the current
/// realm. Unlike indirect `eval`, a script's top-level lexical declarations
/// (`let`/`const`/`class`) become persistent global lexical bindings, so a
/// later declaration of the same name observes them (and var/function
/// declarations reach the global var environment). Used by the Test262 harness.
pub(super) fn native_eval_script(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let Value::String(source) = value else {
        return Ok(value);
    };
    let script = parse_script(&source).map_err(|error| RuntimeError {
        thrown: None,
        message: format!("SyntaxError: {}", error.message),
    })?;
    let bytecode = compile_direct_eval_script(&script, false)?;
    let mut eval_env = env.empty_frame();
    validate_eval_global_lexical_bindings(&bytecode, env, true, true)?;
    // $262.evalScript runs GlobalDeclarationInstantiation: a var/function
    // declaration that cannot be created on a non-extensible global, or that
    // collides with an existing global lexical, is rejected before evaluation.
    let hoisted_function_names = bytecode
        .hoisted_function_names()
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    if !bytecode.is_strict() {
        validate_sloppy_global_eval_declarations(
            &bytecode,
            env,
            &HashSet::new(),
            &hoisted_function_names,
            true,
        )?;
    }
    initialize_eval_script_bindings(&bytecode, &mut eval_env);
    let result = eval_bytecode_with_env(&bytecode, eval_env.clone());
    for name in bytecode.hoisted_local_names() {
        if let Some(value) = result.binding(name) {
            if hoisted_function_names.contains(name) {
                create_eval_script_global_function_binding(&mut eval_env, name, value.clone());
            } else {
                create_eval_script_global_var_binding(&mut eval_env, name, value.clone());
            }
        }
    }
    for name in &result.sloppy_global_names {
        if let Some(value) = result.binding(name) {
            define_eval_global_binding(&mut eval_env, name, value.clone());
        }
    }
    // Top-level lexical declarations persist as global lexical bindings.
    let hoisted = bytecode.hoisted_local_names().collect::<HashSet<_>>();
    for name in bytecode.local_names() {
        if hoisted.contains(name) {
            continue;
        }
        if let Some(value) = result.binding(name) {
            eval_env.insert_realm(name.to_owned(), value.clone());
            eval_env.mark_global_lexical_binding(name.to_owned());
            if bytecode
                .local_slot(name)
                .is_some_and(|slot| !bytecode.local_is_mutable(slot))
            {
                eval_env.mark_immutable_lexical_binding(name.to_owned());
            }
        }
    }
    result.value
}

pub(crate) fn try_eval_regexp_literal_source(
    source: &str,
    env: &CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    let source = source.trim();
    if !source.starts_with('/') || source.starts_with("//") || source.starts_with("/*") {
        return Ok(None);
    }

    let mut in_class = false;
    let mut escaped = false;
    let mut close = None;
    for (index, ch) in source.char_indices().skip(1) {
        if escaped {
            if is_line_terminator(ch) {
                return Ok(None);
            }
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '[' if !in_class => in_class = true,
            ']' if in_class => in_class = false,
            '/' if !in_class => {
                close = Some(index);
                break;
            }
            ch if is_line_terminator(ch) => return Ok(None),
            _ => {}
        }
    }
    let Some(close) = close else {
        return Ok(None);
    };

    let mut flags_end = source.len();
    let mut semicolon = None;
    for (index, ch) in source[close + 1..].char_indices() {
        let absolute = close + 1 + index;
        if ch == ';' {
            flags_end = absolute;
            semicolon = Some(absolute);
            break;
        }
        if ch.is_whitespace() {
            flags_end = absolute;
            break;
        }
        if !ch.is_alphabetic() {
            return Ok(None);
        }
    }

    let rest_start = semicolon.map_or(flags_end, |index| index + 1);
    if !source[rest_start..].trim().is_empty() {
        return Ok(None);
    }

    let pattern = &source[1..close];
    let flags = &source[close + 1..flags_end];
    crate::regexp::regexp_literal_value(pattern, flags, env).map(Some)
}

fn is_line_terminator(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn direct_eval_parse_context(env: &CallEnv) -> EvalParseContext {
    EvalParseContext {
        strict: matches!(
            env.get(crate::DIRECT_EVAL_STRICT_BINDING),
            Some(Value::Boolean(true))
        ),
        in_function: matches!(
            env.get(crate::DIRECT_EVAL_FUNCTION_CONTEXT_BINDING),
            Some(Value::Boolean(true))
        ) || matches!(
            env.get(crate::FIELD_INITIALIZER_EVAL_BINDING),
            Some(Value::Boolean(true))
        ),
        in_method: env.get(crate::HOME_OBJECT_BINDING).is_some(),
        in_derived_constructor: env.get(crate::SUPER_CONSTRUCTOR_BINDING).is_some(),
        in_field_initializer: matches!(
            env.get(crate::FIELD_INITIALIZER_EVAL_BINDING),
            Some(Value::Boolean(true))
        ),
        private_names: env
            .private_environment()
            .map_or_else(Vec::new, |environment| environment.visible_names()),
    }
}

fn validate_eval_global_lexical_bindings(
    bytecode: &crate::bytecode::Bytecode,
    env: &CallEnv,
    include_captured_global_lexicals: bool,
    check_lexical_conflict: bool,
) -> Result<(), RuntimeError> {
    let global_this = env.get(GLOBAL_THIS_BINDING).and_then(|value| match value {
        Value::Object(object) => Some(object),
        _ => None,
    });
    if let Some(global_this) = &global_this {
        for name in bytecode.global_lexical_names() {
            if is_internal_binding_name(name) {
                continue;
            }
            if check_lexical_conflict
                && has_global_lexical_binding(
                    env,
                    global_this,
                    name,
                    include_captured_global_lexicals,
                )
            {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!(
                        "SyntaxError: global lexical declaration `{name}` conflicts with an existing lexical binding"
                    ),
                });
            }
            if global_this
                .own_property(name)
                .is_some_and(|property| !property.configurable)
            {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!(
                        "SyntaxError: global lexical declaration `{name}` conflicts with an existing var binding"
                    ),
                });
            }
        }
    }
    Ok(())
}

fn update_direct_eval_captured_functions(env: &mut CallEnv, name: &str, value: Value) {
    for local_value in env.locals_mut().values_mut() {
        update_function_captured_binding(local_value, name, value.clone());
    }
    if let Some(captured_env) = env.activation_captured_env() {
        let mut captured_env = captured_env.borrow_mut();
        for captured_value in captured_env.values_mut() {
            update_function_captured_binding(captured_value, name, value.clone());
        }
    }
    for captured_env in env.parameter_captured_envs() {
        captured_env
            .borrow_mut()
            .insert(name.to_owned(), value.clone());
    }
}

fn update_direct_eval_parameter_captured_functions(env: &mut CallEnv, name: &str, value: Value) {
    let marker_name = format!(
        "{}{}",
        crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX,
        name
    );
    update_direct_eval_captured_functions(env, name, value.clone());
    if let Some(captured_env) = env.activation_captured_env() {
        let mut captured_env = captured_env.borrow_mut();
        captured_env.insert(name.to_owned(), value);
        captured_env.insert(marker_name.clone(), Value::Boolean(true));
    }
    for captured_env in env.parameter_captured_envs() {
        captured_env
            .borrow_mut()
            .insert(marker_name.clone(), Value::Boolean(true));
    }
}

fn update_function_captured_binding(value: &mut Value, name: &str, replacement: Value) {
    let Value::Function(function) = value else {
        return;
    };
    let mut captured = function.captured_env.borrow_mut();
    if captured.contains_key(name) {
        captured.insert(name.to_owned(), replacement);
    }
}

fn validate_sloppy_function_eval_declarations(
    bytecode: &crate::bytecode::Bytecode,
    env: &CallEnv,
) -> Result<(), RuntimeError> {
    for name in bytecode.hoisted_local_names() {
        if env.is_direct_eval_var_conflict(name) {
            return Err(RuntimeError {
                thrown: None,
                message: format!(
                    "SyntaxError: function eval var declaration `{name}` conflicts with a lexical binding"
                ),
            });
        }
    }
    Ok(())
}

fn validate_sloppy_global_eval_declarations(
    bytecode: &crate::bytecode::Bytecode,
    env: &CallEnv,
    caller_locals: &HashSet<String>,
    function_names: &HashSet<String>,
    include_captured_global_lexicals: bool,
) -> Result<(), RuntimeError> {
    let Some(global_this) = env.get(GLOBAL_THIS_BINDING).and_then(|value| match value {
        Value::Object(object) => Some(object),
        _ => None,
    }) else {
        return Ok(());
    };
    for name in bytecode.hoisted_local_names() {
        if is_internal_binding_name(name) {
            continue;
        }
        if (caller_locals.contains(name)
            && !env.is_catch_binding(name)
            && !global_this.has_own_property(name))
            || has_global_lexical_binding(env, &global_this, name, include_captured_global_lexicals)
        {
            return Err(RuntimeError {
                thrown: None,
                message: format!(
                    "SyntaxError: global var declaration `{name}` conflicts with a global lexical binding"
                ),
            });
        }
    }
    for name in function_names {
        if is_internal_binding_name(name) {
            continue;
        }
        if !can_declare_global_function(&global_this, name) {
            return Err(RuntimeError {
                thrown: None,
                message: format!("TypeError: cannot declare global function `{name}`"),
            });
        }
    }
    for name in bytecode.hoisted_local_names() {
        if is_internal_binding_name(name) {
            continue;
        }
        if function_names.contains(name) {
            continue;
        }
        if !can_declare_global_var(&global_this, name) {
            return Err(RuntimeError {
                thrown: None,
                message: format!("TypeError: cannot declare global var `{name}`"),
            });
        }
    }
    Ok(())
}

fn is_internal_binding_name(name: &str) -> bool {
    name.starts_with('\0')
}

fn has_global_lexical_binding(
    env: &CallEnv,
    global_this: &ObjectRef,
    name: &str,
    include_captured_global_lexicals: bool,
) -> bool {
    if is_marked_dynamic_realm_global(env, global_this) {
        return false;
    }
    !global_this.has_own_property(name)
        && (env.is_global_lexical_binding(name)
            || env.realm_contains(name)
            || (include_captured_global_lexicals
                && (env.locals().contains_key(name) || env.captures_binding(name))))
}

fn can_declare_global_var(global_this: &ObjectRef, name: &str) -> bool {
    global_this.has_own_property(name) || global_this.is_extensible()
}

fn can_declare_global_function(global_this: &ObjectRef, name: &str) -> bool {
    let Some(existing) = global_this.own_property(name) else {
        return global_this.is_extensible();
    };
    existing.configurable || (!existing.accessor && existing.writable && existing.enumerable)
}

fn initialize_direct_eval_bindings(
    bytecode: &crate::bytecode::Bytecode,
    env: &mut CallEnv,
    direct_function_eval: bool,
    direct_parameter_eval: bool,
    caller_locals: &HashSet<String>,
    eval_strict: bool,
) {
    if !env.locals().contains_key("this")
        && let Some(value) = env.get("this")
    {
        env.insert("this".to_owned(), value);
    }
    for name in bytecode.hoisted_local_names() {
        if !eval_strict && caller_locals.contains(name) && !direct_parameter_eval {
            continue;
        }
        if eval_strict {
            env.insert(name.to_owned(), Value::Undefined);
            continue;
        }
        if direct_function_eval {
            if direct_parameter_eval || !env.locals().contains_key(name) {
                env.insert(name.to_owned(), Value::Undefined);
            }
            continue;
        }
        let global_this = env.get(GLOBAL_THIS_BINDING).and_then(|value| match value {
            Value::Object(object) => Some(object),
            _ => None,
        });
        if let Some(property) = global_this
            .as_ref()
            .and_then(|object| object.own_property(name))
        {
            env.insert(name.to_owned(), property.value.clone());
            env.insert_realm(name.to_owned(), property.value);
        } else {
            env.insert(name.to_owned(), Value::Undefined);
            define_eval_global_binding(env, name, Value::Undefined);
        }
    }
}

fn initialize_eval_script_bindings(bytecode: &crate::bytecode::Bytecode, env: &mut CallEnv) {
    if !env.locals().contains_key("this")
        && let Some(value) = env.get("this")
    {
        env.insert("this".to_owned(), value);
    }
    for name in bytecode.hoisted_local_names() {
        let global_this = env.get(GLOBAL_THIS_BINDING).and_then(|value| match value {
            Value::Object(object) => Some(object),
            _ => None,
        });
        if let Some(property) = global_this
            .as_ref()
            .and_then(|object| object.own_property(name))
        {
            env.insert(name.to_owned(), property.value.clone());
            env.insert_realm(name.to_owned(), property.value);
        } else {
            env.insert(name.to_owned(), Value::Undefined);
            create_eval_script_global_var_binding(env, name, Value::Undefined);
        }
    }
}

fn create_eval_global_var_binding(env: &mut CallEnv, name: &str, value: Value) {
    let global_this = env.get(GLOBAL_THIS_BINDING).and_then(|value| match value {
        Value::Object(object) => Some(object),
        _ => None,
    });
    if let Some(global_this) = global_this {
        let dynamic_realm_global = is_marked_dynamic_realm_global(env, &global_this);
        if global_this.has_own_property(name) {
            global_this.set(name.to_owned(), value.clone());
            let value = global_this
                .own_property(name)
                .map(|property| property.value)
                .unwrap_or(value);
            if dynamic_realm_global {
                return;
            }
            env.insert_realm(name.to_owned(), value);
            return;
        }
        global_this.define_property(
            name.to_owned(),
            Property::data(value.clone(), true, true, true),
        );
        if dynamic_realm_global {
            return;
        }
    }
    env.insert_realm(name.to_owned(), value);
}

fn create_eval_script_global_var_binding(env: &mut CallEnv, name: &str, value: Value) {
    let global_this = env.get(GLOBAL_THIS_BINDING).and_then(|value| match value {
        Value::Object(object) => Some(object),
        _ => None,
    });
    if let Some(global_this) = global_this {
        if global_this.has_own_property(name) {
            global_this.set(name.to_owned(), value.clone());
            let value = global_this
                .own_property(name)
                .map(|property| property.value)
                .unwrap_or(value);
            env.insert_realm(name.to_owned(), value);
            return;
        }
        global_this.define_property(
            name.to_owned(),
            Property::data(value.clone(), true, true, false),
        );
    }
    env.insert_realm(name.to_owned(), value);
}

fn create_eval_global_function_binding(env: &mut CallEnv, name: &str, value: Value) {
    let global_this = env.get(GLOBAL_THIS_BINDING).and_then(|value| match value {
        Value::Object(object) => Some(object),
        _ => None,
    });
    if let Some(global_this) = global_this {
        let dynamic_realm_global = is_marked_dynamic_realm_global(env, &global_this);
        let property = match global_this.own_property(name) {
            Some(existing) if !existing.configurable => {
                let mut property = existing;
                property.value = value.clone();
                property
            }
            _ => Property::data(value.clone(), true, true, true),
        };
        global_this.define_property(name.to_owned(), property);
        if dynamic_realm_global {
            return;
        }
    }
    env.insert_realm(name.to_owned(), value);
}

fn create_eval_script_global_function_binding(env: &mut CallEnv, name: &str, value: Value) {
    let global_this = env.get(GLOBAL_THIS_BINDING).and_then(|value| match value {
        Value::Object(object) => Some(object),
        _ => None,
    });
    if let Some(global_this) = global_this {
        let property = match global_this.own_property(name) {
            Some(existing) if !existing.configurable => {
                let mut property = existing;
                property.value = value.clone();
                property
            }
            _ => Property::data(value.clone(), true, true, false),
        };
        global_this.define_property(name.to_owned(), property);
    }
    env.insert_realm(name.to_owned(), value);
}

fn define_eval_global_binding(env: &mut CallEnv, name: &str, value: Value) {
    let global_this = env.get(GLOBAL_THIS_BINDING).and_then(|value| match value {
        Value::Object(object) => Some(object),
        _ => None,
    });
    if let Some(global_this) = global_this {
        let dynamic_realm_global = is_marked_dynamic_realm_global(env, &global_this);
        if global_this.has_own_property(name) {
            global_this.set(name.to_owned(), value.clone());
        } else {
            global_this.define_property(
                name.to_owned(),
                Property::data(value.clone(), true, true, true),
            );
        }
        if dynamic_realm_global {
            return;
        }
    }
    env.insert_realm(name.to_owned(), value);
}

fn is_marked_dynamic_realm_global(env: &CallEnv, global_this: &ObjectRef) -> bool {
    env.get_local(DYNAMIC_FUNCTION_REALM_GLOBAL)
        .is_some_and(|value| matches!(value, Value::Object(global) if global.ptr_eq(global_this)))
        || global_this
            .own_property(DYNAMIC_FUNCTION_REALM_GLOBAL)
            .is_some_and(|property| {
                matches!(property.value, Value::Object(global) if global.ptr_eq(global_this))
            })
}

pub(super) fn native_global_escape(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    let mut escaped = String::new();
    for code_unit in string_code_units(&source) {
        if is_escape_unescaped(code_unit) {
            escaped.push_str(&string_from_code_unit(code_unit));
        } else if code_unit <= 0xFF {
            escaped.push_str(&format!("%{code_unit:02X}"));
        } else {
            escaped.push_str(&format!("%u{code_unit:04X}"));
        }
    }
    Ok(Value::String(escaped.into()))
}

fn is_escape_unescaped(code_unit: u16) -> bool {
    matches!(code_unit, 0x41..=0x5A | 0x61..=0x7A | 0x30..=0x39)
        || matches!(code_unit, 0x40 | 0x2A | 0x5F | 0x2B | 0x2D | 0x2E | 0x2F)
}

pub(super) fn native_global_unescape(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    let mut output = String::new();
    let code_units = string_code_units(&source);
    let mut index = 0;
    while index < code_units.len() {
        if code_units[index] == b'%' as u16 {
            if let Some(code_unit) = parse_hex_escape(&code_units, index) {
                output.push_str(&string_from_code_unit(code_unit));
                index += if code_units.get(index + 1) == Some(&(b'u' as u16)) {
                    6
                } else {
                    3
                };
                continue;
            }
        }
        output.push_str(&string_from_code_unit(code_units[index]));
        index += 1;
    }
    Ok(Value::String(output.into()))
}

fn parse_hex_escape(code_units: &[u16], index: usize) -> Option<u16> {
    if code_units.get(index + 1) == Some(&(b'u' as u16)) {
        return parse_hex_digits(code_units.get(index + 2..index + 6)?);
    }
    parse_hex_digits(code_units.get(index + 1..index + 3)?)
}

fn parse_hex_digits(digits: &[u16]) -> Option<u16> {
    let mut value = 0u16;
    for digit in digits {
        value = value.checked_mul(16)? + u16::try_from(hex_digit(*digit)?).ok()?;
    }
    Some(value)
}

fn hex_digit(code_unit: u16) -> Option<u32> {
    match code_unit {
        0x30..=0x39 => Some(u32::from(code_unit - 0x30)),
        0x61..=0x66 => Some(u32::from(code_unit - 0x61 + 10)),
        0x41..=0x46 => Some(u32::from(code_unit - 0x41 + 10)),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum UriEncodeKind {
    Uri,
    Component,
}

#[derive(Clone, Copy)]
enum UriDecodeKind {
    Uri,
    Component,
}

fn encode_uri(source: &str, kind: UriEncodeKind) -> Result<String, RuntimeError> {
    let mut output = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(character) = chars.next() {
        let Some(code_unit) = surrogate_escape_code_unit(character) else {
            encode_uri_code_point(&mut output, character as u32, kind)?;
            continue;
        };
        let code_point = if is_high_surrogate(code_unit) {
            let Some(low) = chars.next().and_then(surrogate_escape_code_unit) else {
                return malformed_uri();
            };
            if !is_low_surrogate(low) {
                return malformed_uri();
            }
            0x10000 + ((u32::from(code_unit) - 0xD800) << 10) + u32::from(low) - 0xDC00
        } else if is_low_surrogate(code_unit) {
            return malformed_uri();
        } else {
            u32::from(code_unit)
        };
        encode_uri_code_point(&mut output, code_point, kind)?;
    }
    Ok(output)
}

fn encode_uri_code_point(
    output: &mut String,
    code_point: u32,
    kind: UriEncodeKind,
) -> Result<(), RuntimeError> {
    let character = char::from_u32(code_point).ok_or_else(uri_error)?;
    if is_uri_unescaped(character, kind) {
        output.push(character);
    } else {
        let mut buffer = [0; 4];
        for byte in character.encode_utf8(&mut buffer).as_bytes() {
            output.push('%');
            output.push(hex_upper(byte >> 4));
            output.push(hex_upper(byte & 0x0F));
        }
    }
    Ok(())
}

fn decode_uri(source: &str, kind: UriDecodeKind) -> Result<String, RuntimeError> {
    if !source.contains('%') {
        return Ok(source.to_owned());
    }
    if source.is_ascii() {
        return decode_ascii_uri(source, kind);
    }

    let mut output = String::new();
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != '%' {
            output.push(chars[index]);
            index += 1;
            continue;
        }

        let escape_start = index;
        let first_byte = percent_byte(&chars, index)?;
        index += 3;

        let expected_len = utf8_sequence_len(first_byte)?;
        let mut bytes = vec![first_byte];
        for _ in 1..expected_len {
            if index >= chars.len() || chars[index] != '%' {
                return malformed_uri();
            }
            bytes.push(percent_byte(&chars, index)?);
            index += 3;
        }

        let decoded = std::str::from_utf8(&bytes).map_err(|_| uri_error())?;
        if matches!(kind, UriDecodeKind::Uri) && decoded.chars().all(is_uri_reserved) {
            output.extend(chars[escape_start..index].iter());
        } else {
            output.push_str(decoded);
        }
    }
    Ok(output)
}

fn decode_ascii_uri(source: &str, kind: UriDecodeKind) -> Result<String, RuntimeError> {
    let bytes = source.as_bytes();
    let mut output = String::with_capacity(source.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            output.push(char::from(bytes[index]));
            index += 1;
            continue;
        }

        let escape_start = index;
        let first_byte = ascii_percent_byte(bytes, index)?;
        index += 3;

        let expected_len = utf8_sequence_len(first_byte)?;
        let mut decoded_bytes = [0u8; 4];
        decoded_bytes[0] = first_byte;
        for slot in decoded_bytes.iter_mut().take(expected_len).skip(1) {
            if index >= bytes.len() || bytes[index] != b'%' {
                return malformed_uri();
            }
            *slot = ascii_percent_byte(bytes, index)?;
            index += 3;
        }

        let decoded =
            std::str::from_utf8(&decoded_bytes[..expected_len]).map_err(|_| uri_error())?;
        if matches!(kind, UriDecodeKind::Uri) && decoded.chars().all(is_uri_reserved) {
            output.push_str(&source[escape_start..index]);
        } else {
            output.push_str(decoded);
        }
    }
    Ok(output)
}

fn ascii_percent_byte(bytes: &[u8], index: usize) -> Result<u8, RuntimeError> {
    let Some(high) = bytes.get(index + 1).and_then(|byte| ascii_hex_digit(*byte)) else {
        return malformed_uri();
    };
    let Some(low) = bytes.get(index + 2).and_then(|byte| ascii_hex_digit(*byte)) else {
        return malformed_uri();
    };
    Ok((high << 4) | low)
}

fn ascii_hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn is_uri_unescaped(character: char, kind: UriEncodeKind) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(
            character,
            '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')'
        )
        || (matches!(kind, UriEncodeKind::Uri) && is_uri_reserved(character))
}

fn is_uri_reserved(character: char) -> bool {
    matches!(
        character,
        ';' | '/' | '?' | ':' | '@' | '&' | '=' | '+' | '$' | ',' | '#'
    )
}

fn is_high_surrogate(code_unit: u16) -> bool {
    (0xD800..=0xDBFF).contains(&code_unit)
}

fn is_low_surrogate(code_unit: u16) -> bool {
    (0xDC00..=0xDFFF).contains(&code_unit)
}

fn percent_byte(chars: &[char], index: usize) -> Result<u8, RuntimeError> {
    let Some(high) = chars.get(index + 1).and_then(|ch| ch.to_digit(16)) else {
        return malformed_uri();
    };
    let Some(low) = chars.get(index + 2).and_then(|ch| ch.to_digit(16)) else {
        return malformed_uri();
    };
    Ok(((high << 4) | low) as u8)
}

fn utf8_sequence_len(first_byte: u8) -> Result<usize, RuntimeError> {
    match first_byte {
        0x00..=0x7F => Ok(1),
        0xC2..=0xDF => Ok(2),
        0xE0..=0xEF => Ok(3),
        0xF0..=0xF4 => Ok(4),
        _ => malformed_uri(),
    }
}

fn hex_upper(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        10..=15 => char::from(b'A' + nibble - 10),
        _ => unreachable!("nibble must be in 0..16"),
    }
}

fn malformed_uri<T>() -> Result<T, RuntimeError> {
    Err(uri_error())
}

fn uri_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "URIError: malformed URI sequence".to_owned(),
    }
}
