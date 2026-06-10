use qjs_lexer::{Token, TokenKind};

use crate::{ParseError, Parser};

impl Parser {
    pub(crate) fn new(tokens: Vec<Token>, source: String) -> Self {
        Self {
            source,
            tokens,
            cursor: 0,
            strict: false,
            allow_in: true,
            in_method: false,
            in_derived_constructor: false,
        }
    }

    pub(crate) fn at(&self, kind: &TokenKind) -> bool {
        self.peek().is_some_and(|token| token.kind == *kind)
    }

    pub(crate) fn match_kind(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.cursor += 1;
            return true;
        }
        false
    }

    pub(crate) fn match_contextual_keyword(&mut self, keyword: &str) -> bool {
        if matches!(self.peek(), Some(Token { kind: TokenKind::Identifier(name), .. }) if name == keyword)
        {
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
