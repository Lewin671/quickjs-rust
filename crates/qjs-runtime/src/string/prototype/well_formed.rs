use std::collections::HashMap;

use crate::{RuntimeError, Value};

use super::super::indexing::this_string_value;
use super::super::{string_code_units, surrogate_escape_code_unit};
use crate::CallEnv;

pub(crate) fn native_string_prototype_is_well_formed(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    Ok(Value::Boolean(is_well_formed(&value)))
}

pub(crate) fn native_string_prototype_to_well_formed(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    Ok(Value::String(to_well_formed(&value)))
}

fn is_well_formed(value: &str) -> bool {
    let code_units = string_code_units(value);
    let mut index = 0;
    while index < code_units.len() {
        let unit = code_units[index];
        if (0xD800..=0xDBFF).contains(&unit) {
            if index + 1 >= code_units.len() || !(0xDC00..=0xDFFF).contains(&code_units[index + 1])
            {
                return false;
            }
            index += 2;
        } else if (0xDC00..=0xDFFF).contains(&unit) {
            return false;
        } else {
            index += 1;
        }
    }
    true
}

fn to_well_formed(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars().peekable();
    while let Some(character) = chars.next() {
        let Some(unit) = surrogate_escape_code_unit(character) else {
            result.push(character);
            continue;
        };
        if (0xD800..=0xDBFF).contains(&unit) {
            if let Some(next) = chars.peek().copied()
                && let Some(next_unit) = surrogate_escape_code_unit(next)
                && (0xDC00..=0xDFFF).contains(&next_unit)
            {
                result.push(character);
                result.push(next);
                chars.next();
            } else {
                result.push(char::REPLACEMENT_CHARACTER);
            }
        } else {
            result.push(char::REPLACEMENT_CHARACTER);
        }
    }
    result
}
