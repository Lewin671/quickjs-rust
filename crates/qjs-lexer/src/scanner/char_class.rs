pub(super) fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphabetic()
}

pub(super) fn is_identifier_continue(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

pub(super) fn is_ecmascript_whitespace(ch: char) -> bool {
    ch.is_whitespace() || ch == '\u{feff}'
}
