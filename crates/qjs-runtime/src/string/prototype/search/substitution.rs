use crate::Value;

pub(super) fn substitution_length(
    replacement: &str,
    matched: &str,
    position: usize,
    input: &str,
    captures: &[Value],
) -> Option<usize> {
    let mut length = 0usize;
    let matched_len = matched.chars().count();
    let input_len = input.chars().count();
    let suffix_len = input_len.saturating_sub(position + matched_len);
    let capture_lengths: Vec<_> = captures
        .iter()
        .map(|capture| match capture {
            Value::String(capture) => capture.chars().count(),
            _ => 0,
        })
        .collect();
    let mut chars = replacement.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '$' {
            length = length.checked_add(1)?;
            continue;
        }
        let Some(next) = chars.next() else {
            length = length.checked_add(1)?;
            break;
        };
        match next {
            '$' => length = length.checked_add(1)?,
            '&' => length = length.checked_add(matched_len)?,
            '`' => length = length.checked_add(position)?,
            '\'' => length = length.checked_add(suffix_len)?,
            '0'..='9' => {
                length = length.checked_add(capture_substitution_length(
                    &mut chars,
                    &capture_lengths,
                    next,
                )?)?
            }
            _ => length = length.checked_add(2)?,
        }
    }
    Some(length)
}

pub(super) fn get_substitution(
    replacement: &str,
    matched: &str,
    position: usize,
    input: &str,
    captures: &[Value],
) -> String {
    let mut result = String::new();
    let mut chars = replacement.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '$' {
            result.push(character);
            continue;
        }
        let Some(next) = chars.next() else {
            result.push('$');
            break;
        };
        match next {
            '$' => result.push('$'),
            '&' => result.push_str(matched),
            '`' => result.push_str(&input_char_slice(input, 0, position)),
            '\'' => result.push_str(&input_char_slice(
                input,
                position + matched.chars().count(),
                input.chars().count(),
            )),
            '0'..='9' => substitute_capture(&mut result, &mut chars, captures, next),
            _ => {
                result.push('$');
                result.push(next);
            }
        }
    }
    result
}

fn capture_substitution_length(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    capture_lengths: &[usize],
    first_digit: char,
) -> Option<usize> {
    let first = first_digit.to_digit(10).unwrap() as usize;
    if let Some(second) = chars.peek().and_then(|value| value.to_digit(10)) {
        let two_digit = first * 10 + second as usize;
        if (1..=capture_lengths.len()).contains(&two_digit) {
            chars.next();
            return capture_lengths.get(two_digit - 1).copied();
        }
        if first == 0 {
            chars.next();
            return Some(3);
        }
    }
    if (1..=capture_lengths.len()).contains(&first) {
        capture_lengths.get(first - 1).copied()
    } else {
        Some(2)
    }
}

fn substitute_capture(
    result: &mut String,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    captures: &[Value],
    first_digit: char,
) {
    let first = first_digit.to_digit(10).unwrap() as usize;
    if let Some(second) = chars.peek().and_then(|value| value.to_digit(10)) {
        let two_digit = first * 10 + second as usize;
        if (1..=captures.len()).contains(&two_digit) {
            chars.next();
            push_capture(result, captures, two_digit);
            return;
        }
        if first == 0 {
            chars.next();
            result.push('$');
            result.push(first_digit);
            result.push(char::from_digit(second, 10).unwrap());
            return;
        }
    }
    if (1..=captures.len()).contains(&first) {
        push_capture(result, captures, first);
    } else {
        result.push('$');
        result.push(first_digit);
    }
}

fn push_capture(result: &mut String, captures: &[Value], index: usize) {
    if let Some(Value::String(capture)) = captures.get(index - 1) {
        result.push_str(capture);
    }
}

fn input_char_slice(input: &str, start: usize, end: usize) -> String {
    input.chars().skip(start).take(end - start).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitution_length_handles_large_capture_references_without_expansion() {
        let matched = "1".repeat(1 << 15);
        let replacement = "$1".repeat(1 << 16);
        let captures = [Value::String(matched.clone().into())];

        assert_eq!(
            substitution_length(&replacement, &matched, 0, &matched, &captures),
            Some(1usize << 31)
        );
    }
}
