use crate::TokenKind;

use super::Lexer;

impl Lexer<'_> {
    pub(super) fn plus(&mut self) {
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

    pub(super) fn minus(&mut self) {
        if self.html_close_comment() {
            return;
        }

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

    pub(super) fn star(&mut self) {
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

    pub(super) fn percent(&mut self) {
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

    pub(super) fn equal(&mut self) {
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

    pub(super) fn bang(&mut self) {
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

    pub(super) fn less(&mut self) {
        if self.html_open_comment() {
            return;
        }

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

    pub(super) fn greater(&mut self) {
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

    pub(super) fn ampersand(&mut self) {
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

    pub(super) fn pipe(&mut self) {
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

    pub(super) fn caret(&mut self) {
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

    pub(super) fn dot(&mut self) {
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

    pub(super) fn question(&mut self) {
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
}
