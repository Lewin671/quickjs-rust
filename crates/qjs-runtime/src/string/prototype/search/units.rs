//! Non-ASCII search/split and replacement ranges in UTF-16 coordinates.
//! Sentinel-free needles use canonical byte search; only half-pair-capable needles
//! decode temporary unit buffers. Repeated byte matches advance one unit cursor.

use super::kmp::UnitPattern;
use super::{CallEnv, JsString, Replacement, RuntimeError, StringMatch, Value};
use crate::string::{
    code_units, string_code_unit_len, string_code_units, string_from_code_unit,
    string_from_code_units, surrogate_escape_code_unit,
};

// A sentinel-free needle cannot start/end at half a scalar. Canonical storage
// therefore makes byte and unit matches identical, even in a sentinel-bearing
// subject. Actual U+F0000..F07FF scalars contain sentinels and use unit search.
fn byte_searchable(search: &str) -> bool {
    !search
        .chars()
        .any(|ch| surrogate_escape_code_unit(ch).is_some())
}

// Round a unit bound down to a stored character boundary. Forward search rounds
// up when it lands inside a pair; reverse search bounds the match's end instead.
fn byte_floor(input: &str, position: usize) -> (usize, usize) {
    let mut units = 0;
    for (byte, ch) in input.char_indices() {
        let width = if surrogate_escape_code_unit(ch).is_some() {
            1
        } else {
            ch.len_utf16()
        };
        if units + width > position {
            return (byte, units);
        }
        units += width;
    }
    (input.len(), units)
}

pub(super) fn find(input: &str, search: &str, start: usize) -> Option<usize> {
    if !search.is_empty() && byte_searchable(search) {
        let (mut byte, mut units) = byte_floor(input, start);
        if units < start {
            let ch = input[byte..].chars().next()?;
            byte += ch.len_utf8();
            units += ch.len_utf16();
        }
        let offset = input[byte..].find(search)?;
        return Some(units + string_code_unit_len(&input[byte..byte + offset]));
    }
    UnitPattern::new(search).find_in(&string_code_units(input), start, false)
}

pub(super) fn rfind(input: &str, search: &str, position: usize) -> Option<usize> {
    if !search.is_empty() && byte_searchable(search) {
        let end = position.saturating_add(string_code_unit_len(search));
        let (byte, _) = byte_floor(input, end);
        let offset = input[..byte].rfind(search)?;
        return Some(string_code_unit_len(&input[..offset]));
    }
    let input = string_code_units(input);
    let search = UnitPattern::new(search);
    let end = position.saturating_add(search.units.len()).min(input.len());
    search.find_in(&input[..end], 0, true)
}

pub(super) fn starts_with(input: &str, search: &str, start: usize) -> bool {
    let mut input = code_units(input).skip(start);
    code_units(search).all(|unit| input.next() == Some(unit))
}

pub(super) fn ends_with(input: &str, search: &str, end: usize) -> bool {
    let length = crate::string::string_code_unit_len(search);
    end.checked_sub(length)
        .is_some_and(|start| starts_with(input, search, start))
}

pub(in super::super) fn split_code_units(input: &str, separator: &str, limit: usize) -> Vec<Value> {
    if separator.is_empty() {
        return code_units(input)
            .take(limit)
            .map(|unit| Value::String(string_from_code_unit(unit).into()))
            .collect();
    }
    if byte_searchable(separator) {
        return input
            .split(separator)
            .take(limit)
            .map(|part| Value::String(part.to_owned().into()))
            .collect();
    }
    let input = string_code_units(input);
    let separator = UnitPattern::new(separator);
    let mut parts = Vec::new();
    let mut start = 0;
    while parts.len() < limit {
        let Some(position) = separator.find_in(&input, start, false) else {
            break;
        };
        parts.push(Value::String(
            string_from_code_units(&input[start..position]).into(),
        ));
        start = position + separator.units.len();
    }
    if parts.len() < limit {
        parts.push(Value::String(
            string_from_code_units(&input[start..]).into(),
        ));
    }
    parts
}

pub(super) fn match_positions(input: &str, search: &str, first_only: bool) -> Vec<StringMatch> {
    if !search.is_empty() && byte_searchable(search) {
        let length = string_code_unit_len(search);
        let mut byte_cursor = 0;
        let mut unit_cursor = 0;
        return input
            .match_indices(search)
            .take(if first_only { 1 } else { usize::MAX })
            .map(|(byte, _)| {
                let start = unit_cursor + string_code_unit_len(&input[byte_cursor..byte]);
                byte_cursor = byte + search.len();
                unit_cursor = start + length;
                StringMatch {
                    start,
                    end: unit_cursor,
                    matched: search.to_owned(),
                    captures: Vec::new(),
                }
            })
            .collect();
    }
    let input = string_code_units(input);
    let needle = UnitPattern::new(search);
    let mut matches = Vec::new();
    let mut start = 0;
    while let Some(position) = needle.find_in(&input, start, false) {
        let end = position + needle.units.len();
        matches.push(StringMatch {
            start: position,
            end,
            matched: search.to_owned(),
            captures: Vec::new(),
        });
        if first_only {
            break;
        }
        start = if needle.units.is_empty() {
            end + 1
        } else {
            end
        };
    }
    matches
}

pub(super) fn replace_matches(
    input: JsString,
    matches: Vec<StringMatch>,
    replacement: Replacement,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let units = string_code_units(&input);
    if let Replacement::String(replacement) = &replacement
        && !replacement.contains('$')
    {
        super::preflight_literal_replacement_length(units.len(), &matches, replacement)?;
    }
    let mut result = String::new();
    let mut length = 0;
    let mut copied_until = 0;
    for matched in matches {
        // Generic exec results may report an out-of-range or overlapping index.
        let start = matched.start.min(units.len());
        let end = matched.end.min(units.len());
        if start < copied_until {
            continue;
        }
        length = super::checked_string_length(length, start - copied_until)?;
        crate::string::push_js_str(
            &mut result,
            &string_from_code_units(&units[copied_until..start]),
        );
        let text = match &replacement {
            Replacement::Function(function) => {
                super::functional_replacement((**function).clone(), &matched, input.clone(), env)?
            }
            Replacement::String(replacement) => {
                let added = super::substitution_length(
                    replacement,
                    &matched.matched,
                    matched.start,
                    units.len(),
                    &matched.captures,
                )
                .ok_or_else(super::invalid_string_length)?;
                // Check before expanding potentially large prefix/suffix substitutions.
                super::checked_string_length(length, added)?;
                super::get_substitution(
                    replacement,
                    &matched.matched,
                    matched.start,
                    &input,
                    units.len(),
                    &matched.captures,
                )
            }
        };
        length = super::checked_string_length(length, crate::string::string_code_unit_len(&text))?;
        crate::string::push_js_str(&mut result, &text);
        copied_until = end;
    }
    super::checked_string_length(length, units.len() - copied_until)?;
    crate::string::push_js_str(&mut result, &string_from_code_units(&units[copied_until..]));
    Ok(Value::String(result.into()))
}

#[cfg(test)]
#[path = "units_tests.rs"]
mod tests;
