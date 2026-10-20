//! Non-ASCII search/split and replacement ranges in UTF-16 coordinates.
//! Temporary unit buffers belong to one operation; no cached views or ASCII work.

use super::{CallEnv, JsString, Replacement, RuntimeError, StringMatch, Value};
use crate::string::{code_units, string_code_units, string_from_code_unit, string_from_code_units};

pub(super) fn find(input: &str, search: &str, start: usize) -> Option<usize> {
    let input = string_code_units(input);
    let search = string_code_units(search);
    find_in(&input, &search, start)
}

fn find_in(input: &[u16], search: &[u16], start: usize) -> Option<usize> {
    if search.is_empty() {
        return (start <= input.len()).then_some(start);
    }
    input
        .get(start..)?
        .windows(search.len())
        .position(|part| part == search)
        .map(|offset| start + offset)
}

pub(super) fn rfind(input: &str, search: &str, position: usize) -> Option<usize> {
    let input = string_code_units(input);
    let search = string_code_units(search);
    if search.is_empty() {
        return Some(position.min(input.len()));
    }
    let last = position.min(input.len().checked_sub(search.len())?);
    input
        .windows(search.len())
        .take(last + 1)
        .rposition(|part| part == search)
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
    let input = string_code_units(input);
    let separator = string_code_units(separator);
    let mut parts = Vec::new();
    let mut start = 0;
    while parts.len() < limit {
        let Some(position) = find_in(&input, &separator, start) else {
            break;
        };
        parts.push(Value::String(
            string_from_code_units(&input[start..position]).into(),
        ));
        start = position + separator.len();
    }
    if parts.len() < limit {
        parts.push(Value::String(
            string_from_code_units(&input[start..]).into(),
        ));
    }
    parts
}

pub(super) fn match_positions(input: &str, search: &str, first_only: bool) -> Vec<StringMatch> {
    let input = string_code_units(input);
    let needle = string_code_units(search);
    let mut matches = Vec::new();
    let mut start = 0;
    while let Some(position) = find_in(&input, &needle, start) {
        let end = position + needle.len();
        matches.push(StringMatch {
            start: position,
            end,
            matched: search.to_owned(),
            captures: Vec::new(),
        });
        if first_only {
            break;
        }
        start = if needle.is_empty() { end + 1 } else { end };
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
mod tests {
    use super::*;

    #[test]
    fn search_ranges_include_interior_pair_boundaries() {
        let low = string_from_code_unit(0xDE00);
        let high = string_from_code_unit(0xD83D);
        assert_eq!(find("😀x😀", &low, 0), Some(1));
        assert_eq!(find("😀x😀", &low, 2), Some(4));
        assert_eq!(rfind("😀x😀", &high, 2), Some(0));
        assert_eq!(rfind("😀x😀", &low, 4), Some(4));
        assert!(starts_with("😀x", &low, 1));
        assert!(ends_with("😀x", &high, 1));
        assert_eq!(find("😀", "", 2), Some(2));
        assert_eq!(find("😀", "", 3), None);
    }
}
