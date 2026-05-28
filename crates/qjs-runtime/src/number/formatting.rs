pub(crate) fn number_to_js_string(number: f64) -> String {
    if number.is_nan() {
        "NaN".to_owned()
    } else if number == f64::INFINITY {
        "Infinity".to_owned()
    } else if number == f64::NEG_INFINITY {
        "-Infinity".to_owned()
    } else if number == 0.0 {
        "0".to_owned()
    } else if number.fract() == 0.0 {
        format!("{number:.0}")
    } else {
        number.to_string()
    }
}
