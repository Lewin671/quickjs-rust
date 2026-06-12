use crate::CallEnv;
use std::borrow::Cow;

use crate::{RuntimeError, Value, to_js_string_with_env};
use unicode_normalization::UnicodeNormalization;

use super::super::indexing::this_string_value;
use super::super::{string_code_units, string_from_code_unit};

pub(crate) fn native_string_prototype_to_lower_case(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(case_convert(
        &this_string_value(this_value, env)?,
        str::to_lowercase,
    )))
}

pub(crate) fn native_string_prototype_to_upper_case(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(case_convert(
        &this_string_value(this_value, env)?,
        str::to_uppercase,
    )))
}

pub(crate) fn native_string_prototype_locale_compare(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let that = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;

    let value = canonical_locale_compare_key(&value);
    let that = canonical_locale_compare_key(&that);
    let result = match value.as_ref().cmp(that.as_ref()) {
        std::cmp::Ordering::Less => -1.0,
        std::cmp::Ordering::Equal => 0.0,
        std::cmp::Ordering::Greater => 1.0,
    };
    Ok(Value::Number(result))
}

pub(crate) fn native_string_prototype_normalize(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let form = match argument_values.first() {
        None | Some(Value::Undefined) => "NFC".to_owned(),
        Some(value) => to_js_string_with_env(value.clone(), env)?,
    };
    let normalized = match form.as_str() {
        "NFC" => normalize_string(&value, |value| value.nfc().collect()),
        "NFD" => normalize_string(&value, |value| value.nfd().collect()),
        "NFKC" => normalize_string(&value, |value| value.nfkc().collect()),
        "NFKD" => normalize_string(&value, |value| value.nfkd().collect()),
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "RangeError: invalid normalization form".to_owned(),
            });
        }
    };
    Ok(Value::String(normalized))
}

fn canonical_locale_compare_key(value: &str) -> Cow<'_, str> {
    if value.is_ascii() {
        Cow::Borrowed(value)
    } else {
        Cow::Owned(value.nfc().collect())
    }
}

fn normalize_string(value: &str, normalize: impl FnOnce(&str) -> String) -> String {
    if value.is_ascii() {
        value.to_owned()
    } else {
        normalize(value)
    }
}

fn case_convert(value: &str, convert: impl FnOnce(&str) -> String) -> String {
    let code_points = decode_utf16_code_points(value);
    encode_utf16_code_units(&convert(&code_points))
}

fn decode_utf16_code_points(value: &str) -> String {
    let code_units = string_code_units(value);
    let mut result = String::new();
    let mut index = 0;
    while index < code_units.len() {
        let first = code_units[index];
        if (0xD800..=0xDBFF).contains(&first) && index + 1 < code_units.len() {
            let second = code_units[index + 1];
            if (0xDC00..=0xDFFF).contains(&second) {
                let code_point =
                    (u32::from(first) - 0xD800) * 1024 + (u32::from(second) - 0xDC00) + 0x10000;
                if let Some(character) = char::from_u32(code_point) {
                    result.push(character);
                    index += 2;
                    continue;
                }
            }
        }

        result.push_str(&string_from_code_unit(first));
        index += 1;
    }
    result
}

fn encode_utf16_code_units(value: &str) -> String {
    string_code_units(value)
        .into_iter()
        .map(string_from_code_unit)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{Value, eval};

    #[test]
    fn case_conversion_iterates_over_utf16_code_points() {
        assert_eq!(
            eval("'\\uD801\\uDC00'.toLowerCase() === '\\uD801\\uDC28';"),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval("'\\uD801\\uDC28'.toUpperCase() === '\\uD801\\uDC00';"),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval("'\\uD835\\uDCA2\\u03A3'.toLowerCase() === '\\uD835\\uDCA2\\u03C2';"),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval("'A\\u03A3\\uD835\\uDCA2'.toLowerCase() === 'a\\u03C3\\uD835\\uDCA2';"),
            Ok(Value::Boolean(true))
        );
    }

    #[test]
    fn locale_compare_returns_ordering_number() {
        assert_eq!(
            eval("'abc'.localeCompare('abc') === 0;"),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval("'abc'.localeCompare('abd') < 0;"),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval("'abd'.localeCompare('abc') > 0;"),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval("'undefined'.localeCompare() === 0;"),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(r#"'o\u0308'.localeCompare('ö') === 0;"#),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(r#"'Å'.localeCompare('A\u030A') === 0;"#),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(r#"'가'.localeCompare('\u1100\u1161') === 0;"#),
            Ok(Value::Boolean(true))
        );
    }

    #[test]
    fn normalize_supports_unicode_forms() {
        assert_eq!(
            eval(r#"'o\u0308'.normalize() === 'ö';"#),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(r#"'ö'.normalize('NFD') === 'o\u0308';"#),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(r#"'\uFB00'.normalize('NFKC') === 'ff';"#),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(r#"'\u1E9B\u0323'.normalize('NFKD') === 's\u0323\u0307';"#),
            Ok(Value::Boolean(true))
        );
    }

    #[test]
    fn normalize_rejects_invalid_form() {
        assert_eq!(
            eval(
                "let caught = false; try { 'x'.normalize('bad'); } catch (error) { caught = error instanceof RangeError; } caught;"
            ),
            Ok(Value::Boolean(true))
        );
    }
}
