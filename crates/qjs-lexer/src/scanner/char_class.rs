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
        // WhiteSpace
        '\u{0009}'  // TAB
            | '\u{000B}'  // VT
            | '\u{000C}'  // FF
            | '\u{0020}'  // SPACE
            | '\u{00A0}'  // NBSP
            | '\u{FEFF}'  // ZWNBSP (BOM)
            | '\u{1680}'  // OGHAM SPACE MARK
            | '\u{2000}'  // EN QUAD
            | '\u{2001}'  // EM QUAD
            | '\u{2002}'  // EN SPACE
            | '\u{2003}'  // EM SPACE
            | '\u{2004}'  // THREE-PER-EM SPACE
            | '\u{2005}'  // FOUR-PER-EM SPACE
            | '\u{2006}'  // SIX-PER-EM SPACE
            | '\u{2007}'  // FIGURE SPACE
            | '\u{2008}'  // PUNCTUATION SPACE
            | '\u{2009}'  // THIN SPACE
            | '\u{200A}'  // HAIR SPACE
            | '\u{202F}'  // NARROW NO-BREAK SPACE
            | '\u{205F}'  // MEDIUM MATHEMATICAL SPACE
            | '\u{3000}'  // IDEOGRAPHIC SPACE
            // LineTerminator
            | '\u{000A}'  // LF
            | '\u{000D}'  // CR
            | '\u{2028}'  // LS
            | '\u{2029}' // PS
    )
}
