use std::collections::HashMap;

use crate::{RuntimeError, Value, to_js_string, to_length};

use super::super::indexing::this_string_value;

#[derive(Clone, Copy)]
pub(crate) enum StringPadKind {
    Start,
    End,
}

pub(crate) fn native_string_prototype_pad(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
    kind: StringPadKind,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let max_length = to_length(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let string_length = value.chars().count();
    if max_length <= string_length {
        return Ok(Value::String(value));
    }

    let fill_string = match argument_values.get(1).cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => " ".to_owned(),
        value => to_js_string(value)?,
    };
    if fill_string.is_empty() {
        return Ok(Value::String(value));
    }

    let fill_length = max_length - string_length;
    let filler = repeated_prefix(&fill_string, fill_length);
    Ok(Value::String(match kind {
        StringPadKind::Start => format!("{filler}{value}"),
        StringPadKind::End => format!("{value}{filler}"),
    }))
}

fn repeated_prefix(pattern: &str, length: usize) -> String {
    pattern.chars().cycle().take(length).collect()
}
