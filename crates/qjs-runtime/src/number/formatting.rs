pub(crate) fn number_to_js_string(number: f64) -> String {
    if number.is_nan() {
        "NaN".to_owned()
    } else if number == f64::INFINITY {
        "Infinity".to_owned()
    } else if number == f64::NEG_INFINITY {
        "-Infinity".to_owned()
    } else if number == 0.0 {
        "0".to_owned()
    } else if number.abs() >= 1e21 || number.abs() < 1e-6 {
        to_js_exponential_string(number)
    } else if number.fract() == 0.0 {
        format!("{number:.0}")
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
