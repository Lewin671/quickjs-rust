use std::collections::HashMap;

use qjs_parser::parse_script;

use crate::{
    Function, NativeFunction, Property, RuntimeError, Value,
    bytecode::{compile_script, eval_bytecode_with_env},
    string::{string_code_units, string_from_code_unit},
    to_js_string_with_env, to_number,
};

pub(super) fn install_globals(env: &mut HashMap<String, Value>, global_this: &Value) {
    env.insert("NaN".to_owned(), Value::Number(f64::NAN));
    env.insert("Infinity".to_owned(), Value::Number(f64::INFINITY));
    if let Value::Object(global_object) = global_this {
        global_object.define_property(
            "NaN".to_owned(),
            Property::data(Value::Number(f64::NAN), false, false, false),
        );
        global_object.define_property(
            "Infinity".to_owned(),
            Property::data(Value::Number(f64::INFINITY), false, false, false),
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
    define_global_function(env, global_this, "eval", 1, NativeFunction::Eval);
    define_global_function(env, global_this, "escape", 1, NativeFunction::Escape);
    define_global_function(env, global_this, "unescape", 1, NativeFunction::Unescape);
}

fn define_global_function(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    key: &str,
    length: usize,
    native: NativeFunction,
) {
    let value = Value::Function(Function::new_native(Some(key), length, native, false));
    env.insert(key.to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set(key.to_owned(), value);
    }
}

pub(super) fn native_global_is_finite(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(to_number(value)?.is_finite()))
}

pub(super) fn native_global_is_nan(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(to_number(value)?.is_nan()))
}

pub(super) fn native_global_eval(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let Value::String(source) = value else {
        return Ok(value);
    };
    let script = parse_script(&source).map_err(|error| RuntimeError {
        thrown: None,
        message: error.message,
    })?;
    let bytecode = compile_script(&script)?;
    let result = eval_bytecode_with_env(&bytecode, env.clone());
    for name in bytecode
        .local_names()
        .chain(bytecode.global_names().iter().map(String::as_str))
    {
        if let Some(value) = result.binding(name) {
            env.insert(name.to_owned(), value.clone());
        }
    }
    result.value
}

pub(super) fn native_global_escape(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
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
    Ok(Value::String(escaped))
}

fn is_escape_unescaped(code_unit: u16) -> bool {
    matches!(code_unit, 0x41..=0x5A | 0x61..=0x7A | 0x30..=0x39)
        || matches!(code_unit, 0x40 | 0x2A | 0x5F | 0x2B | 0x2D | 0x2E | 0x2F)
}

pub(super) fn native_global_unescape(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
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
    Ok(Value::String(output))
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
