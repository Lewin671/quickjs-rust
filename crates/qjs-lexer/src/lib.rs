//! Tokenization for the Rust QuickJS rewrite.

use qjs_ast::Span;

/// A token with its source span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    /// Token kind.
    pub kind: TokenKind,
    /// Source span.
    pub span: Span,
}

/// Token categories recognized by the lexer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenKind {
    /// Identifier text.
    Identifier(String),
    /// Number literal text.
    Number(String),
    /// String literal value.
    String(String),
    /// `true`.
    True,
    /// `false`.
    False,
    /// `null`.
    Null,
    /// `+`.
    Plus,
    /// `-`.
    Minus,
    /// `*`.
    Star,
    /// `/`.
    Slash,
    /// `%`.
    Percent,
    /// `=`.
    Equal,
    /// `!`.
    Bang,
    /// `<`.
    Less,
    /// `>`.
    Greater,
    /// `&`.
    Ampersand,
    /// `|`.
    Pipe,
    /// `^`.
    Caret,
    /// `~`.
    Tilde,
    /// `(`.
    LeftParen,
    /// `)`.
    RightParen,
    /// `{`.
    LeftBrace,
    /// `}`.
    RightBrace,
    /// `[`.
    LeftBracket,
    /// `]`.
    RightBracket,
    /// `,`.
    Comma,
    /// `.`.
    Dot,
    /// `:`.
    Colon,
    /// `?`.
    Question,
    /// `;`.
    Semicolon,
    /// End of input.
    Eof,
}

/// A lexer error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LexError {
    /// Human-readable message.
    pub message: String,
    /// Source span.
    pub span: Span,
}

/// Lexes JavaScript source into tokens.
///
/// # Errors
///
/// Returns a `LexError` when an unsupported character or unterminated string is
/// encountered.
pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).lex()
}

struct Lexer<'src> {
    source: &'src str,
    cursor: usize,
    tokens: Vec<Token>,
}

impl<'src> Lexer<'src> {
    fn new(source: &'src str) -> Self {
        Self {
            source,
            cursor: 0,
            tokens: Vec::new(),
        }
    }

    fn lex(mut self) -> Result<Vec<Token>, LexError> {
        while let Some(ch) = self.peek() {
            match ch {
                c if c.is_ascii_whitespace() => {
                    self.advance();
                }
                c if is_identifier_start(c) => self.identifier(),
                c if c.is_ascii_digit() => self.number(),
                '"' | '\'' => self.string(ch)?,
                '+' => self.single(TokenKind::Plus),
                '-' => self.single(TokenKind::Minus),
                '*' => self.single(TokenKind::Star),
                '/' => self.slash_or_comment()?,
                '%' => self.single(TokenKind::Percent),
                '=' => self.single(TokenKind::Equal),
                '!' => self.single(TokenKind::Bang),
                '<' => self.single(TokenKind::Less),
                '>' => self.single(TokenKind::Greater),
                '&' => self.single(TokenKind::Ampersand),
                '|' => self.single(TokenKind::Pipe),
                '^' => self.single(TokenKind::Caret),
                '~' => self.single(TokenKind::Tilde),
                '(' => self.single(TokenKind::LeftParen),
                ')' => self.single(TokenKind::RightParen),
                '{' => self.single(TokenKind::LeftBrace),
                '}' => self.single(TokenKind::RightBrace),
                '[' => self.single(TokenKind::LeftBracket),
                ']' => self.single(TokenKind::RightBracket),
                ',' => self.single(TokenKind::Comma),
                '.' => self.single(TokenKind::Dot),
                ':' => self.single(TokenKind::Colon),
                '?' => self.single(TokenKind::Question),
                ';' => self.single(TokenKind::Semicolon),
                _ => {
                    let start = self.cursor;
                    self.advance();
                    return Err(LexError {
                        message: format!("unsupported character `{ch}`"),
                        span: Span::new(start, self.cursor),
                    });
                }
            }
        }

        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(self.cursor, self.cursor),
        });
        Ok(self.tokens)
    }

    fn slash_or_comment(&mut self) -> Result<(), LexError> {
        let start = self.cursor;
        self.advance();

        match self.peek() {
            Some('/') => {
                self.advance();
                while !matches!(self.peek(), None | Some('\n' | '\r')) {
                    self.advance();
                }
                Ok(())
            }
            Some('*') => {
                self.advance();
                self.block_comment(start)
            }
            _ => {
                self.tokens.push(Token {
                    kind: TokenKind::Slash,
                    span: Span::new(start, self.cursor),
                });
                Ok(())
            }
        }
    }

    fn block_comment(&mut self, start: usize) -> Result<(), LexError> {
        while let Some(ch) = self.advance() {
            if ch == '*' && self.peek() == Some('/') {
                self.advance();
                return Ok(());
            }
        }

        Err(LexError {
            message: "unterminated block comment".to_owned(),
            span: Span::new(start, self.cursor),
        })
    }

    fn identifier(&mut self) {
        let start = self.cursor;
        while matches!(self.peek(), Some(ch) if is_identifier_continue(ch)) {
            self.advance();
        }
        let text = &self.source[start..self.cursor];
        let kind = match text {
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            _ => TokenKind::Identifier(text.to_owned()),
        };
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.cursor),
        });
    }

    fn number(&mut self) {
        let start = self.cursor;
        while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
            self.advance();
        }
        if self.peek() == Some('.') {
            self.advance();
            while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
                self.advance();
            }
        }
        self.tokens.push(Token {
            kind: TokenKind::Number(self.source[start..self.cursor].to_owned()),
            span: Span::new(start, self.cursor),
        });
    }

    fn string(&mut self, quote: char) -> Result<(), LexError> {
        let start = self.cursor;
        self.advance();
        let content_start = self.cursor;
        while let Some(ch) = self.peek() {
            if ch == quote {
                let value = self.source[content_start..self.cursor].to_owned();
                self.advance();
                self.tokens.push(Token {
                    kind: TokenKind::String(value),
                    span: Span::new(start, self.cursor),
                });
                return Ok(());
            }
            self.advance();
        }

        Err(LexError {
            message: "unterminated string literal".to_owned(),
            span: Span::new(start, self.cursor),
        })
    }

    fn single(&mut self, kind: TokenKind) {
        let start = self.cursor;
        self.advance();
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.cursor),
        });
    }

    fn peek(&self) -> Option<char> {
        self.source[self.cursor..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.cursor += ch.len_utf8();
        Some(ch)
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use qjs_ast::Span;

    use super::{TokenKind, lex};

    #[test]
    fn lexes_expression() {
        let tokens = lex("answer + 42;").expect("source should lex");
        let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Identifier("answer".to_owned()),
                TokenKind::Plus,
                TokenKind::Number("42".to_owned()),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn skips_line_and_block_comments() {
        let tokens = lex("one // ignore\n/* skip */ two").expect("source should lex");
        let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Identifier("one".to_owned()),
                TokenKind::Identifier("two".to_owned()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn reports_unterminated_block_comment() {
        let error = lex("1 /* unfinished").expect_err("comment should fail");
        assert_eq!(error.message, "unterminated block comment");
        assert_eq!(error.span, Span::new(2, 15));
    }

    #[test]
    fn lexes_common_punctuators_with_spans() {
        let tokens = lex("{}[](),.:?%=!<>|&^~").expect("source should lex");
        let actual: Vec<_> = tokens
            .into_iter()
            .map(|token| (token.kind, token.span))
            .collect();
        assert_eq!(
            actual,
            vec![
                (TokenKind::LeftBrace, Span::new(0, 1)),
                (TokenKind::RightBrace, Span::new(1, 2)),
                (TokenKind::LeftBracket, Span::new(2, 3)),
                (TokenKind::RightBracket, Span::new(3, 4)),
                (TokenKind::LeftParen, Span::new(4, 5)),
                (TokenKind::RightParen, Span::new(5, 6)),
                (TokenKind::Comma, Span::new(6, 7)),
                (TokenKind::Dot, Span::new(7, 8)),
                (TokenKind::Colon, Span::new(8, 9)),
                (TokenKind::Question, Span::new(9, 10)),
                (TokenKind::Percent, Span::new(10, 11)),
                (TokenKind::Equal, Span::new(11, 12)),
                (TokenKind::Bang, Span::new(12, 13)),
                (TokenKind::Less, Span::new(13, 14)),
                (TokenKind::Greater, Span::new(14, 15)),
                (TokenKind::Pipe, Span::new(15, 16)),
                (TokenKind::Ampersand, Span::new(16, 17)),
                (TokenKind::Caret, Span::new(17, 18)),
                (TokenKind::Tilde, Span::new(18, 19)),
                (TokenKind::Eof, Span::new(19, 19)),
            ]
        );
    }
}
