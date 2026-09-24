pub(crate) fn number_to_js_string(number: f64) -> String {
    if number.is_nan() {
        "NaN".to_owned()
    } else if number == f64::INFINITY {
        "Infinity".to_owned()
    } else if number == f64::NEG_INFINITY {
        "-Infinity".to_owned()
    } else if number == 0.0 {
        "0".to_owned()
    } else if number.fract() == 0.0 && number.abs() < 1e15 {
        integer_to_string(number as i64)
    } else if number.abs() >= 1e21 || number.abs() < 1e-6 {
        to_js_exponential_string(number)
    } else {
        number.to_string()
    }
}

fn to_js_exponential_string(number: f64) -> String {
    let formatted = format!("{number:e}");
    let Some((mantissa, exponent)) = formatted.split_once('e') else {
        return formatted;
    };
    let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
    let exponent = if let Some(unsigned) = exponent.strip_prefix('-') {
        format!("-{}", unsigned.trim_start_matches('0'))
    } else {
        format!("+{}", exponent.trim_start_matches('0'))
    };
    format!("{mantissa}e{exponent}")
}

/// An integral number's digits, written directly rather than through the
/// shortest-round-trip float formatter, which is exact for these values but
/// several times slower.
fn integer_to_string(value: i64) -> String {
    let mut text = String::with_capacity(21);
    push_integer(&mut text, value);
    text
}

fn push_integer(out: &mut String, value: i64) {
    let mut digits = [0_u8; 20];
    let mut position = digits.len();
    let mut rest = value.unsigned_abs();
    loop {
        position -= 1;
        digits[position] = b'0' + (rest % 10) as u8;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    if value < 0 {
        out.push('-');
    }
    for &digit in &digits[position..] {
        out.push(char::from(digit));
    }
}

/// Appends `number`'s ToString to `out`: an integer's digits and an ordinary
/// fraction are written in place, with no string of their own -- string
/// building and `JSON.stringify` format a number per value.
pub(crate) fn push_number_js_string(out: &mut String, number: f64) {
    if number.is_finite() && number != 0.0 {
        let magnitude = number.abs();
        if number.fract() == 0.0 && magnitude < 1e15 {
            push_integer(out, number as i64);
            return;
        }
        if (1e-6..1e21).contains(&magnitude) {
            use std::fmt::Write;
            let _ = write!(out, "{number}");
            return;
        }
    }
    out.push_str(&number_to_js_string(number));
}
