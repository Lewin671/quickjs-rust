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
    /// `var`.
    Var,
    /// `let`.
    Let,
    /// `const`.
    Const,
    /// `if`.
    If,
    /// `else`.
    Else,
    /// `while`.
    While,
    /// `do`.
    Do,
    /// `for`.
    For,
    /// `break`.
    Break,
    /// `continue`.
    Continue,
    /// `function`.
    Function,
    /// `return`.
    Return,
    /// `throw`.
    Throw,
    /// `typeof`.
    Typeof,
    /// `in`.
    In,
    /// `delete`.
    Delete,
    /// `+`.
    Plus,
    /// `++`.
    PlusPlus,
    /// `+=`.
    PlusEqual,
    /// `-`.
    Minus,
    /// `--`.
    MinusMinus,
    /// `-=`.
    MinusEqual,
    /// `=>`.
    Arrow,
    /// `*`.
    Star,
    /// `**`.
    StarStar,
    /// `*=`.
    StarEqual,
    /// `**=`.
    StarStarEqual,
    /// `/`.
    Slash,
    /// `/=`.
    SlashEqual,
    /// `%`.
    Percent,
    /// `%=`.
    PercentEqual,
    /// `=`.
    Equal,
    /// `==`.
    EqualEqual,
    /// `===`.
    EqualEqualEqual,
    /// `!`.
    Bang,
    /// `!=`.
    BangEqual,
    /// `!==`.
    BangEqualEqual,
    /// `<`.
    Less,
    /// `<=`.
    LessEqual,
    /// `<<`.
    LessLess,
    /// `<<=`.
    LessLessEqual,
    /// `>`.
    Greater,
    /// `>=`.
    GreaterEqual,
    /// `>>`.
    GreaterGreater,
    /// `>>=`.
    GreaterGreaterEqual,
    /// `>>>`.
    GreaterGreaterGreater,
    /// `>>>=`.
    GreaterGreaterGreaterEqual,
    /// `&`.
    Ampersand,
    /// `&&`.
    AmpersandAmpersand,
    /// `&=`.
    AmpersandEqual,
    /// `&&=`.
    AmpersandAmpersandEqual,
    /// `|`.
    Pipe,
    /// `||`.
    PipePipe,
    /// `|=`.
    PipeEqual,
    /// `||=`.
    PipePipeEqual,
    /// `^`.
    Caret,
    /// `^=`.
    CaretEqual,
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
    /// `...`.
    DotDotDot,
    /// `:`.
    Colon,
    /// `?`.
    Question,
    /// `??`.
    QuestionQuestion,
    /// `?.`.
    QuestionDot,
    /// `??=`.
    QuestionQuestionEqual,
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
                '+' => self.plus(),
                '-' => self.minus(),
                '*' => self.star(),
                '/' => self.slash_or_comment()?,
                '%' => self.percent(),
                '=' => self.equal(),
                '!' => self.bang(),
                '<' => self.less(),
                '>' => self.greater(),
                '&' => self.ampersand(),
                '|' => self.pipe(),
                '^' => self.caret(),
                '~' => self.single(TokenKind::Tilde),
                '(' => self.single(TokenKind::LeftParen),
                ')' => self.single(TokenKind::RightParen),
                '{' => self.single(TokenKind::LeftBrace),
                '}' => self.single(TokenKind::RightBrace),
                '[' => self.single(TokenKind::LeftBracket),
                ']' => self.single(TokenKind::RightBracket),
                ',' => self.single(TokenKind::Comma),
                '.' => self.dot(),
                ':' => self.single(TokenKind::Colon),
                '?' => self.question(),
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

    fn plus(&mut self) {
        let start = self.cursor;
        self.advance();
        let kind = match self.peek() {
            Some('+') => {
                self.advance();
                TokenKind::PlusPlus
            }
            Some('=') => {
                self.advance();
                TokenKind::PlusEqual
            }
            _ => TokenKind::Plus,
        };
        self.push(kind, start);
    }

    fn minus(&mut self) {
        let start = self.cursor;
        self.advance();
        let kind = match self.peek() {
            Some('-') => {
                self.advance();
                TokenKind::MinusMinus
            }
            Some('=') => {
                self.advance();
                TokenKind::MinusEqual
            }
            _ => TokenKind::Minus,
        };
        self.push(kind, start);
    }

    fn star(&mut self) {
        let start = self.cursor;
        self.advance();
        let kind = match self.peek() {
            Some('*') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::StarStarEqual
                } else {
                    TokenKind::StarStar
                }
            }
            Some('=') => {
                self.advance();
                TokenKind::StarEqual
            }
            _ => TokenKind::Star,
        };
        self.push(kind, start);
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
                if self.peek() == Some('=') {
                    self.advance();
                    self.push(TokenKind::SlashEqual, start);
                } else {
                    self.push(TokenKind::Slash, start);
                }
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

    fn percent(&mut self) {
        let start = self.cursor;
        self.advance();
        let kind = if self.peek() == Some('=') {
            self.advance();
            TokenKind::PercentEqual
        } else {
            TokenKind::Percent
        };
        self.push(kind, start);
    }

    fn equal(&mut self) {
        let start = self.cursor;
        self.advance();
        let kind = match self.peek() {
            Some('=') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::EqualEqualEqual
                } else {
                    TokenKind::EqualEqual
                }
            }
            Some('>') => {
                self.advance();
                TokenKind::Arrow
            }
            _ => TokenKind::Equal,
        };
        self.push(kind, start);
    }

    fn bang(&mut self) {
        let start = self.cursor;
        self.advance();
        let kind = if self.peek() == Some('=') {
            self.advance();
            if self.peek() == Some('=') {
                self.advance();
                TokenKind::BangEqualEqual
            } else {
                TokenKind::BangEqual
            }
        } else {
            TokenKind::Bang
        };
        self.push(kind, start);
    }

    fn less(&mut self) {
        let start = self.cursor;
        self.advance();
        let kind = match self.peek() {
            Some('<') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::LessLessEqual
                } else {
                    TokenKind::LessLess
                }
            }
            Some('=') => {
                self.advance();
                TokenKind::LessEqual
            }
            _ => TokenKind::Less,
        };
        self.push(kind, start);
    }

    fn greater(&mut self) {
        let start = self.cursor;
        self.advance();
        let kind = match self.peek() {
            Some('>') => {
                self.advance();
                if self.peek() == Some('>') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::GreaterGreaterGreaterEqual
                    } else {
                        TokenKind::GreaterGreaterGreater
                    }
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::GreaterGreaterEqual
                } else {
                    TokenKind::GreaterGreater
                }
            }
            Some('=') => {
                self.advance();
                TokenKind::GreaterEqual
            }
            _ => TokenKind::Greater,
        };
        self.push(kind, start);
    }

    fn ampersand(&mut self) {
        let start = self.cursor;
        self.advance();
        let kind = match self.peek() {
            Some('&') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::AmpersandAmpersandEqual
                } else {
                    TokenKind::AmpersandAmpersand
                }
            }
            Some('=') => {
                self.advance();
                TokenKind::AmpersandEqual
            }
            _ => TokenKind::Ampersand,
        };
        self.push(kind, start);
    }

    fn pipe(&mut self) {
        let start = self.cursor;
        self.advance();
        let kind = match self.peek() {
            Some('|') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::PipePipeEqual
                } else {
                    TokenKind::PipePipe
                }
            }
            Some('=') => {
                self.advance();
                TokenKind::PipeEqual
            }
            _ => TokenKind::Pipe,
        };
        self.push(kind, start);
    }

    fn caret(&mut self) {
        let start = self.cursor;
        self.advance();
        let kind = if self.peek() == Some('=') {
            self.advance();
            TokenKind::CaretEqual
        } else {
            TokenKind::Caret
        };
        self.push(kind, start);
    }

    fn dot(&mut self) {
        let start = self.cursor;
        self.advance();
        let kind = if self.peek() == Some('.') && self.peek_nth(1) == Some('.') {
            self.advance();
            self.advance();
            TokenKind::DotDotDot
        } else {
            TokenKind::Dot
        };
        self.push(kind, start);
    }

    fn question(&mut self) {
        let start = self.cursor;
        self.advance();
        let kind = match self.peek() {
            Some('?') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::QuestionQuestionEqual
                } else {
                    TokenKind::QuestionQuestion
                }
            }
            Some('.') => {
                self.advance();
                TokenKind::QuestionDot
            }
            _ => TokenKind::Question,
        };
        self.push(kind, start);
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
            "var" => TokenKind::Var,
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "do" => TokenKind::Do,
            "for" => TokenKind::For,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "function" => TokenKind::Function,
            "return" => TokenKind::Return,
            "throw" => TokenKind::Throw,
            "typeof" => TokenKind::Typeof,
            "in" => TokenKind::In,
            "delete" => TokenKind::Delete,
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
        self.push(kind, start);
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.cursor),
        });
    }

    fn peek(&self) -> Option<char> {
        self.source[self.cursor..].chars().next()
    }

    fn peek_nth(&self, n: usize) -> Option<char> {
        self.source[self.cursor..].chars().nth(n)
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
    fn lexes_declaration_keywords() {
        let tokens =
            lex(
                "var let const if else while do for break continue function return throw typeof in delete variable",
            )
            .expect("source should lex");
        let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Var,
                TokenKind::Let,
                TokenKind::Const,
                TokenKind::If,
                TokenKind::Else,
                TokenKind::While,
                TokenKind::Do,
                TokenKind::For,
                TokenKind::Break,
                TokenKind::Continue,
                TokenKind::Function,
                TokenKind::Return,
                TokenKind::Throw,
                TokenKind::Typeof,
                TokenKind::In,
                TokenKind::Delete,
                TokenKind::Identifier("variable".to_owned()),
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
        let tokens = lex("{}[](),.:?%!<>|&^~=").expect("source should lex");
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
                (TokenKind::Bang, Span::new(11, 12)),
                (TokenKind::Less, Span::new(12, 13)),
                (TokenKind::Greater, Span::new(13, 14)),
                (TokenKind::Pipe, Span::new(14, 15)),
                (TokenKind::Ampersand, Span::new(15, 16)),
                (TokenKind::Caret, Span::new(16, 17)),
                (TokenKind::Tilde, Span::new(17, 18)),
                (TokenKind::Equal, Span::new(18, 19)),
                (TokenKind::Eof, Span::new(19, 19)),
            ]
        );
    }

    #[test]
    fn lexes_multi_character_punctuators_with_longest_match() {
        let tokens = lex(
            "++ += -- -= => ** **= *= /= %= == === != !== <= << <<= >= >> >>= >>> >>>= && &&= &= || ||= |= ^= ... ?? ??= ?.",
        )
        .expect("source should lex");
        let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::PlusPlus,
                TokenKind::PlusEqual,
                TokenKind::MinusMinus,
                TokenKind::MinusEqual,
                TokenKind::Arrow,
                TokenKind::StarStar,
                TokenKind::StarStarEqual,
                TokenKind::StarEqual,
                TokenKind::SlashEqual,
                TokenKind::PercentEqual,
                TokenKind::EqualEqual,
                TokenKind::EqualEqualEqual,
                TokenKind::BangEqual,
                TokenKind::BangEqualEqual,
                TokenKind::LessEqual,
                TokenKind::LessLess,
                TokenKind::LessLessEqual,
                TokenKind::GreaterEqual,
                TokenKind::GreaterGreater,
                TokenKind::GreaterGreaterEqual,
                TokenKind::GreaterGreaterGreater,
                TokenKind::GreaterGreaterGreaterEqual,
                TokenKind::AmpersandAmpersand,
                TokenKind::AmpersandAmpersandEqual,
                TokenKind::AmpersandEqual,
                TokenKind::PipePipe,
                TokenKind::PipePipeEqual,
                TokenKind::PipeEqual,
                TokenKind::CaretEqual,
                TokenKind::DotDotDot,
                TokenKind::QuestionQuestion,
                TokenKind::QuestionQuestionEqual,
                TokenKind::QuestionDot,
                TokenKind::Eof,
            ]
        );
    }
}
