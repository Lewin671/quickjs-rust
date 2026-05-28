use qjs_ast::Span;

use crate::{LexError, Token, TokenKind};

pub(crate) struct Lexer<'src> {
    source: &'src str,
    cursor: usize,
    tokens: Vec<Token>,
}

impl<'src> Lexer<'src> {
    pub(crate) fn new(source: &'src str) -> Self {
        Self {
            source,
            cursor: 0,
            tokens: Vec::new(),
        }
    }

    pub(crate) fn lex(mut self) -> Result<Vec<Token>, LexError> {
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
            "return" => TokenKind::Return,
            "throw" => TokenKind::Throw,
            "debugger" => TokenKind::Debugger,
            "typeof" => TokenKind::Typeof,
            "void" => TokenKind::Void,
            "in" => TokenKind::In,
            "delete" => TokenKind::Delete,
            "new" => TokenKind::New,
            "instanceof" => TokenKind::Instanceof,
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
