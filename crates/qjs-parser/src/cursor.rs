use qjs_lexer::{Token, TokenKind};

use crate::{ParseError, Parser};

impl Parser {
    pub(crate) fn new(tokens: Vec<Token>, source: String, source_is_wtf16: bool) -> Self {
        let closers = matching_closers(&tokens);
        Self {
            source,
            source_is_wtf16,
            tokens,
            closers,
            cursor: 0,
            goal: crate::Goal::Script,
            strict: false,
            allow_in: true,
            in_method: false,
            in_derived_constructor: false,
            in_field_initializer: false,
            in_function: false,
            allow_return: false,
            in_static_block: false,
            in_generator: false,
            in_generator_params: false,
            in_async: false,
            in_async_params: false,
            private_scopes: Vec::new(),
            next_private_scope_id: 0,
            pending_private_refs: Vec::new(),
        }
    }

    pub(crate) fn at(&self, kind: &TokenKind) -> bool {
        // Every precedence level asks this about each of its operators for
        // every operand, and nearly every answer is no: comparing the variant
        // first answers those without the out-of-line derived comparison.
        self.peek().is_some_and(|token| {
            std::mem::discriminant(&token.kind) == std::mem::discriminant(kind)
                && token.kind == *kind
        })
    }

    pub(crate) fn match_kind(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.cursor += 1;
            return true;
        }
        false
    }

    pub(crate) fn match_contextual_keyword(&mut self, keyword: &str) -> bool {
        // A contextual keyword written with a Unicode escape (e.g. `\u{6f}f`)
        // does not play its syntactic role: per ECMA-262 a keyword's meaning
        // requires its literal spelling, so an escaped spelling stays a plain
        // identifier here.
        if matches!(
            self.peek(),
            Some(Token { kind: TokenKind::Identifier(name), had_escape: false, .. }) if name == keyword
        ) {
            self.cursor += 1;
            return true;
        }
        false
    }

    pub(crate) fn expect(&mut self, kind: &TokenKind) -> Result<(), ParseError> {
        if self.match_kind(kind) {
            Ok(())
        } else {
            let token = self.peek().expect("parser should always have eof token");
            Err(ParseError {
                message: format!("expected `{kind:?}`"),
                span: token.span,
            })
        }
    }

    pub(crate) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    pub(crate) fn peek_nth(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.cursor + offset)
    }

    pub(crate) fn advance(&mut self) -> Token {
        let token = self
            .peek()
            .expect("parser should always have eof token")
            .clone();
        self.cursor += 1;
        token
    }
}

/// Marks a token that closes nothing, or an opener without a closer.
pub(crate) const NO_CLOSER: u32 = u32::MAX;

/// The index of the token closing each bracket token, found in one pass.
fn matching_closers(tokens: &[Token]) -> Vec<u32> {
    let mut closers = vec![NO_CLOSER; tokens.len()];
    let mut open: Vec<usize> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        let opener = match token.kind {
            TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::LeftBrace => {
                open.push(index);
                continue;
            }
            TokenKind::RightParen => TokenKind::LeftParen,
            TokenKind::RightBracket => TokenKind::LeftBracket,
            TokenKind::RightBrace => TokenKind::LeftBrace,
            _ => continue,
        };
        if let Some(&start) = open.last()
            && tokens[start].kind == opener
        {
            open.pop();
            if let Ok(close) = u32::try_from(index) {
                closers[start] = close;
            }
        }
    }
    closers
}
