use std::collections::HashMap;

use crate::{RuntimeError, Value};

use super::super::indexing::this_string_value;
use super::super::{string_code_units, string_from_code_unit};

pub(crate) fn native_string_prototype_is_well_formed(
    this_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    Ok(Value::Boolean(is_well_formed(&value)))
}

pub(crate) fn native_string_prototype_to_well_formed(
    this_value: Value,
    env: &mut HashMap<String, Value>,
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
    let code_units = string_code_units(value);
    let mut result = String::new();
    let mut index = 0;
    while index < code_units.len() {
        let unit = code_units[index];
        if (0xD800..=0xDBFF).contains(&unit) {
            if index + 1 < code_units.len() && (0xDC00..=0xDFFF).contains(&code_units[index + 1]) {
                result.push_str(&string_from_code_unit(unit));
                result.push_str(&string_from_code_unit(code_units[index + 1]));
                index += 2;
            } else {
                result.push(char::REPLACEMENT_CHARACTER);
                index += 1;
            }
        } else if (0xDC00..=0xDFFF).contains(&unit) {
            result.push(char::REPLACEMENT_CHARACTER);
            index += 1;
        } else {
            result.push_str(&string_from_code_unit(unit));
            index += 1;
        }
    }
    result
}
