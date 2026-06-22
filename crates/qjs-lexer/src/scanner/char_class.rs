/// Reports whether `ch` may begin an `IdentifierName`.
///
/// The ASCII fast path (`_`, `$`, letters) covers the common case. Non-ASCII
/// characters use the Unicode 17.0.0 `ID_Start` table shared with RegExp
/// group-name validation.
pub(super) fn is_identifier_start(ch: char) -> bool {
    if ch.is_ascii() {
        return ch == '_' || ch == '$' || ch.is_ascii_alphabetic();
    }
    qjs_unicode::is_id_start(ch as u32)
}

/// Reports whether `ch` may continue an `IdentifierName`.
///
/// The ASCII fast path covers the common case. Non-ASCII characters use the
/// Unicode 17.0.0 `ID_Continue` table plus the ECMAScript ZWNJ/ZWJ additions.
pub(super) fn is_identifier_continue(ch: char) -> bool {
    if ch.is_ascii() {
        return ch == '_' || ch == '$' || ch.is_ascii_alphanumeric();
    }
    qjs_unicode::is_id_continue(ch as u32) || matches!(ch, '\u{200C}' | '\u{200D}')
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
