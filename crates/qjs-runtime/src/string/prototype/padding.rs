use crate::{RuntimeError, Value, to_js_string_with_env, to_length_with_env};

use super::super::indexing::this_string_value;
use crate::CallEnv;

#[derive(Clone, Copy)]
pub(crate) enum StringPadKind {
    Start,
    End,
}

pub(crate) fn native_string_prototype_pad(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
    kind: StringPadKind,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let max_length = to_length_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let string_length = value.chars().count();
    if max_length <= string_length {
        return Ok(Value::String(value.into()));
    }
    // The padded result must not exceed the maximum string length; QuickJS-NG
    // throws rather than attempt a multi-gigabyte allocation.
    if max_length > super::MAX_STRING_LENGTH {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid string length".to_owned(),
        });
    }

    let fill_string = match argument_values.get(1).cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => " ".to_owned(),
        value => to_js_string_with_env(value, env)?,
    };
    if fill_string.is_empty() {
        return Ok(Value::String(value.into()));
    }

    let fill_length = max_length - string_length;
    let filler = repeated_prefix(&fill_string, fill_length);
    Ok(Value::String(
        match kind {
            StringPadKind::Start => format!("{filler}{value}"),
            StringPadKind::End => format!("{value}{filler}"),
        }
        .into(),
    ))
}

fn repeated_prefix(pattern: &str, length: usize) -> String {
    pattern.chars().cycle().take(length).collect()
}
