/// Reports whether `ch` may begin an `IdentifierName`.
///
/// The ASCII fast path (`_`, `$`, letters) covers the common case. For
/// non-ASCII we approximate UAX#31 `ID_Start` with [`char::is_alphabetic`]
/// plus the two spec-mandated additions `U+2118`, `U+212E` and the
/// `Other_ID_Start` characters. This is an approximation: the full UAX#31
/// `ID_Start` set (which excludes some letters and includes `Letter_Number`
/// `Nl` code points such as Roman numerals) is intentionally out of scope for
/// this slice, so a handful of exotic code points may be accepted or rejected
/// incorrectly. ECMAScript ZWNJ/ZWJ handling in identifiers is likewise not
/// modeled here.
pub(super) fn is_identifier_start(ch: char) -> bool {
    if ch.is_ascii() {
        return ch == '_' || ch == '$' || ch.is_ascii_alphabetic();
    }
    ch.is_alphabetic() || matches!(ch, '\u{2118}' | '\u{212E}' | '\u{309B}' | '\u{309C}')
}

/// Reports whether `ch` may continue an `IdentifierName`.
///
/// Approximates UAX#31 `ID_Continue` with [`char::is_alphanumeric`] plus the
/// ASCII digit/`_`/`$` fast path. As with [`is_identifier_start`], combining
/// marks (`Mn`/`Mc`), connector punctuation beyond `_`, and ZWNJ/ZWJ are not
/// fully modeled; that exactness is out of scope for this slice.
pub(super) fn is_identifier_continue(ch: char) -> bool {
    if ch.is_ascii() {
        return ch == '_' || ch == '$' || ch.is_ascii_alphanumeric();
    }
    ch.is_alphanumeric()
        || matches!(
            ch,
            '\u{2118}' | '\u{212E}' | '\u{309B}' | '\u{309C}' | '\u{200C}' | '\u{200D}'
        )
}

pub(super) fn is_js_whitespace_or_line_terminator(ch: char) -> bool {
    matches!(
        ch,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{2028}'
            | '\u{2029}'
    )
}
