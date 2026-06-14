use qjs_lexer::TokenKind;

pub(crate) fn has_legacy_octal_escape(raw: &str) -> bool {
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            continue;
        }
        let Some(escaped) = chars.next() else {
            return false;
        };
        match escaped {
            '0' => {
                if matches!(chars.peek(), Some('0'..='9')) {
                    return true;
                }
            }
            '1'..='7' => return true,
            _ => {}
        }
    }
    false
}

pub(crate) fn is_legacy_octal_or_non_octal_decimal_literal(raw: &str) -> bool {
    let cleaned: String = raw.chars().filter(|&ch| ch != '_').collect();
    cleaned.len() > 1
        && cleaned.starts_with('0')
        && cleaned.bytes().all(|byte| byte.is_ascii_digit())
}

pub(crate) fn keyword_property_name(kind: &TokenKind) -> Option<&'static str> {
    match kind {
        TokenKind::This => Some("this"),
        TokenKind::Var => Some("var"),
        TokenKind::Let => Some("let"),
        TokenKind::Const => Some("const"),
        TokenKind::If => Some("if"),
        TokenKind::Else => Some("else"),
        TokenKind::While => Some("while"),
        TokenKind::Do => Some("do"),
        TokenKind::For => Some("for"),
        TokenKind::Switch => Some("switch"),
        TokenKind::Case => Some("case"),
        TokenKind::Default => Some("default"),
        TokenKind::Try => Some("try"),
        TokenKind::Catch => Some("catch"),
        TokenKind::Finally => Some("finally"),
        TokenKind::Break => Some("break"),
        TokenKind::Continue => Some("continue"),
        TokenKind::Function => Some("function"),
        TokenKind::Class => Some("class"),
        TokenKind::Extends => Some("extends"),
        TokenKind::Super => Some("super"),
        TokenKind::Return => Some("return"),
        TokenKind::Throw => Some("throw"),
        TokenKind::Debugger => Some("debugger"),
        TokenKind::Typeof => Some("typeof"),
        TokenKind::Void => Some("void"),
        TokenKind::In => Some("in"),
        TokenKind::With => Some("with"),
        TokenKind::Delete => Some("delete"),
        TokenKind::New => Some("new"),
        TokenKind::Instanceof => Some("instanceof"),
        _ => None,
    }
}
