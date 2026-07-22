use qjs_ast::Span;

use crate::{LexError, LexOptions, Token, TokenKind};

mod char_class;
mod comments;
mod keywords;
mod literals;
mod punctuators;

use char_class::{is_identifier_start, is_js_whitespace_or_line_terminator};

/// Runtime strings use real Rust scalar values except for lone UTF-16
/// surrogates, which occupy this private-use sentinel range. A real scalar in
/// the same range must therefore be expanded to its UTF-16 pair before it
/// enters a string token; otherwise it is indistinguishable from one lone
/// surrogate code unit.
const SURROGATE_ESCAPE_SENTINEL_BASE: u32 = 0xF0000;

pub(in crate::scanner) fn push_js_scalar(result: &mut String, character: char) {
    if (SURROGATE_ESCAPE_SENTINEL_BASE..SURROGATE_ESCAPE_SENTINEL_BASE + 0x800)
        .contains(&(character as u32))
    {
        let mut buffer = [0; 2];
        for code_unit in character.encode_utf16(&mut buffer) {
            push_js_code_unit(result, *code_unit);
        }
    } else {
        result.push(character);
    }
}

pub(in crate::scanner) fn surrogate_escape_code_unit(character: char) -> Option<u16> {
    let code_point = character as u32;
    (SURROGATE_ESCAPE_SENTINEL_BASE..SURROGATE_ESCAPE_SENTINEL_BASE + 0x800)
        .contains(&code_point)
        .then(|| (0xD800 + code_point - SURROGATE_ESCAPE_SENTINEL_BASE) as u16)
}

pub(in crate::scanner) fn push_js_code_unit(result: &mut String, code_unit: u16) {
    if (0xD800..=0xDFFF).contains(&code_unit) {
        let character =
            char::from_u32(SURROGATE_ESCAPE_SENTINEL_BASE + u32::from(code_unit) - 0xD800)
                .expect("surrogate sentinel is a valid scalar value");
        result.push(character);
    } else {
        result.push(char::from_u32(u32::from(code_unit)).expect("BMP code unit is a valid scalar"));
    }
}

pub(crate) struct Lexer<'src> {
    pub(in crate::scanner) source: &'src str,
    pub(in crate::scanner) cursor: usize,
    pub(in crate::scanner) tokens: Vec<Token>,
    pub(in crate::scanner) template_stack: Vec<TemplateState>,
    options: LexOptions,
}

pub(in crate::scanner) struct TemplateState {
    pub(in crate::scanner) brace_depth: usize,
}

impl<'src> Lexer<'src> {
    pub(crate) fn with_options(source: &'src str, options: LexOptions) -> Self {
        Self {
            source,
            cursor: 0,
            tokens: Vec::new(),
            template_stack: Vec::new(),
            options,
        }
    }

    pub(crate) fn lex(mut self) -> Result<Vec<Token>, LexError> {
        while let Some(ch) = self.peek() {
            match ch {
                c if is_js_whitespace_or_line_terminator(c) => {
                    self.advance();
                }
                c if is_identifier_start(c) => self.identifier()?,
                '\\' if self.peek_nth(1) == Some('u') => self.identifier()?,
                c if c.is_ascii_digit() => self.number()?,
                '"' | '\'' => self.string(ch)?,
                '`' => self.template_literal()?,
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
                '\\' => self.single(TokenKind::Backslash),
                '(' => self.single(TokenKind::LeftParen),
                ')' => self.single(TokenKind::RightParen),
                '{' => self.left_brace(),
                '}' if self.template_substitution_is_complete() => {
                    self.template_after_substitution()?;
                }
                '}' => self.right_brace(),
                '[' => self.single(TokenKind::LeftBracket),
                ']' => self.single(TokenKind::RightBracket),
                ',' => self.single(TokenKind::Comma),
                '.' if matches!(self.peek_nth(1), Some(ch) if ch.is_ascii_digit()) => {
                    self.number_starting_with_dot()?;
                }
                '.' => self.dot(),
                ':' => self.single(TokenKind::Colon),
                '?' => self.question(),
                ';' => self.single(TokenKind::Semicolon),
                '#' if self.options.hashbang
                    && self.cursor == 0
                    && self.peek_nth(1) == Some('!') =>
                {
                    self.hashbang_comment();
                }
                '#' => self.private_name()?,
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
            had_escape: false,
        });
        Ok(self.tokens)
    }

    pub(in crate::scanner) fn single(&mut self, kind: TokenKind) {
        let start = self.cursor;
        self.advance();
        self.push(kind, start);
    }

    pub(in crate::scanner) fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.cursor),
            had_escape: false,
        });
    }

    fn left_brace(&mut self) {
        if let Some(template) = self.template_stack.last_mut() {
            template.brace_depth += 1;
        }
        self.single(TokenKind::LeftBrace);
    }

    fn right_brace(&mut self) {
        if let Some(template) = self.template_stack.last_mut() {
            template.brace_depth = template.brace_depth.saturating_sub(1);
        }
        self.single(TokenKind::RightBrace);
    }

    fn template_substitution_is_complete(&self) -> bool {
        self.template_stack
            .last()
            .is_some_and(|template| template.brace_depth == 0)
    }

    pub(in crate::scanner) fn peek(&self) -> Option<char> {
        self.source[self.cursor..].chars().next()
    }

    pub(in crate::scanner) fn peek_nth(&self, n: usize) -> Option<char> {
        self.source[self.cursor..].chars().nth(n)
    }

    pub(in crate::scanner) fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.cursor += ch.len_utf8();
        Some(ch)
    }

    pub(in crate::scanner) fn push_source_character(&self, result: &mut String, character: char) {
        if self.options.wtf16_source && surrogate_escape_code_unit(character).is_some() {
            result.push(character);
        } else {
            push_js_scalar(result, character);
        }
    }

    pub(in crate::scanner) fn push_source_str(&self, result: &mut String, value: &str) {
        for character in value.chars() {
            self.push_source_character(result, character);
        }
    }
}
