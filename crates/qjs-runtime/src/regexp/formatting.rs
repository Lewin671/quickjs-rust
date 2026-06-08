pub(super) fn escape_regexp_source(source: &str) -> String {
    if source.is_empty() {
        return "(?:)".to_owned();
    }
    let mut escaped = String::new();
    for ch in source.chars() {
        match ch {
            '/' => escaped.push_str("\\/"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\u{2028}' => escaped.push_str("\\u2028"),
            '\u{2029}' => escaped.push_str("\\u2029"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub(super) fn canonical_regexp_flags(flags: &str) -> String {
    "dgimsyu"
        .chars()
        .filter(|flag| flags.contains(*flag))
        .collect()
}
