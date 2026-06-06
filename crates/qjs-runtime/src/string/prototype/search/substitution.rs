use crate::Value;

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
