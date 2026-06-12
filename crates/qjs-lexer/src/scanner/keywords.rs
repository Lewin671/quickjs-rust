use crate::TokenKind;

pub(super) fn identifier_or_keyword(text: &str) -> TokenKind {
    match text {
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "null" => TokenKind::Null,
        "this" => TokenKind::This,
        "var" => TokenKind::Var,
        "let" => TokenKind::Let,
        "const" => TokenKind::Const,
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "while" => TokenKind::While,
        "do" => TokenKind::Do,
        "for" => TokenKind::For,
        "switch" => TokenKind::Switch,
        "case" => TokenKind::Case,
        "default" => TokenKind::Default,
        "try" => TokenKind::Try,
        "catch" => TokenKind::Catch,
        "finally" => TokenKind::Finally,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,
        "function" => TokenKind::Function,
        "class" => TokenKind::Class,
        "extends" => TokenKind::Extends,
        "super" => TokenKind::Super,
        "return" => TokenKind::Return,
        "throw" => TokenKind::Throw,
        "debugger" => TokenKind::Debugger,
        "typeof" => TokenKind::Typeof,
        "void" => TokenKind::Void,
        "in" => TokenKind::In,
        "with" => TokenKind::With,
        "delete" => TokenKind::Delete,
        "new" => TokenKind::New,
        "instanceof" => TokenKind::Instanceof,
        _ => TokenKind::Identifier(text.to_owned()),
    }
}

/// Reports whether `text` is an unconditionally reserved word: one that is
/// never a valid `IdentifierName` in any context. These map to a dedicated
/// keyword `TokenKind` in [`identifier_or_keyword`]. Per ECMA-262 11.6.2 such a
/// word may not be written with a `UnicodeEscapeSequence`, so the lexer rejects
/// an escaped spelling that decodes to one of these.
///
/// Contextual and strict-mode-only reserved words (`let`, `static`, `yield`,
/// `await`, `async`, `get`, `set`, `of`, `eval`, `arguments`, the future
/// reserved words) are intentionally excluded here; their escaped-spelling
/// restrictions depend on parser context and are enforced in the parser.
pub(super) fn is_always_reserved_word(text: &str) -> bool {
    // `let` has a dedicated keyword token but is only contextually reserved
    // (it is a valid identifier in sloppy code), so an escaped spelling is not
    // a lex-time error; the parser decides whether the escaped `let` may act
    // as a declaration keyword.
    if text == "let" {
        return false;
    }
    !matches!(identifier_or_keyword(text), TokenKind::Identifier(_))
}
