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
fn lexes_prefixed_numeric_literals() {
    let tokens = lex("0x10 0Xf 0b101 0B11 0o77 0O10").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Number("0x10".to_owned()),
            TokenKind::Number("0Xf".to_owned()),
            TokenKind::Number("0b101".to_owned()),
            TokenKind::Number("0B11".to_owned()),
            TokenKind::Number("0o77".to_owned()),
            TokenKind::Number("0O10".to_owned()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn rejects_invalid_prefixed_numeric_literals() {
    assert!(lex("0xG").is_err());
    assert!(lex("0b2").is_err());
    assert!(lex("0o8").is_err());
    assert!(lex("0x").is_err());
}

#[test]
fn lexes_decimal_exponent_numeric_literals() {
    let tokens = lex("1e3 1E+3 1e-3 1.25e2 .5e1 1.").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Number("1e3".to_owned()),
            TokenKind::Number("1E+3".to_owned()),
            TokenKind::Number("1e-3".to_owned()),
            TokenKind::Number("1.25e2".to_owned()),
            TokenKind::Number(".5e1".to_owned()),
            TokenKind::Number("1.".to_owned()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn rejects_invalid_decimal_exponent_numeric_literals() {
    assert!(lex("1e").is_err());
    assert!(lex("1e+").is_err());
    assert!(lex("1e-").is_err());
    assert!(lex("1e1x").is_err());
    assert!(lex("1abc").is_err());
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
fn lexes_no_substitution_template_literals_as_strings() {
    let tokens = lex("`hello` `` `price $5`").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::String("hello".to_owned()),
            TokenKind::String(String::new()),
            TokenKind::String("price $5".to_owned()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn rejects_template_literals_with_substitution() {
    let error = lex("`hello ${name}`").expect_err("substitution should fail");
    assert_eq!(
        error.message,
        "template substitution is not supported yet".to_owned()
    );
    assert_eq!(error.span, Span::new(0, 9));
}

#[test]
fn reports_unterminated_template_literal() {
    let error = lex("`unfinished").expect_err("template should fail");
    assert_eq!(error.message, "unterminated template literal");
    assert_eq!(error.span, Span::new(0, 11));
}

#[test]
fn lexes_string_escape_sequences() {
    let tokens = lex(r#""\n\t\b\f\r\v\\\"\'\0\x41\u0042\u{43}\A""#).expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::String("\n\t\u{0008}\u{000c}\r\u{000b}\\\"'\0ABC A".replace(" ", "")),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn skips_string_line_continuations() {
    let tokens = lex("\"a\\\nb\"").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![TokenKind::String("ab".to_owned()), TokenKind::Eof]
    );
}

#[test]
fn rejects_invalid_string_escape_sequences() {
    assert!(lex(r#""\xG0""#).is_err());
    assert!(lex(r#""\u00G0""#).is_err());
    assert!(lex(r#""\u{}""#).is_err());
    assert!(lex(r#""\8""#).is_err());
}

#[test]
fn rejects_unescaped_line_terminators_in_strings() {
    let error = lex("\"a\nb\"").expect_err("string should fail");
    assert_eq!(error.message, "unterminated string literal");
}

#[test]
fn lexes_template_escape_sequences() {
    let tokens = lex(r#"`\n\x41\u0042\u{43}\``"#).expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![TokenKind::String("\nABC`".to_owned()), TokenKind::Eof]
    );
}

#[test]
fn lexes_declaration_keywords() {
    let tokens =
            lex(
                "this var let const if else while do for switch case default try catch finally break continue function return throw debugger typeof void in delete new instanceof variable",
            )
            .expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::This,
            TokenKind::Var,
            TokenKind::Let,
            TokenKind::Const,
            TokenKind::If,
            TokenKind::Else,
            TokenKind::While,
            TokenKind::Do,
            TokenKind::For,
            TokenKind::Switch,
            TokenKind::Case,
            TokenKind::Default,
            TokenKind::Try,
            TokenKind::Catch,
            TokenKind::Finally,
            TokenKind::Break,
            TokenKind::Continue,
            TokenKind::Function,
            TokenKind::Return,
            TokenKind::Throw,
            TokenKind::Debugger,
            TokenKind::Typeof,
            TokenKind::Void,
            TokenKind::In,
            TokenKind::Delete,
            TokenKind::New,
            TokenKind::Instanceof,
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
