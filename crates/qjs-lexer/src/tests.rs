use qjs_ast::Span;

use super::{LexOptions, TemplateSegment, Token, TokenKind, lex, lex_with_options};

fn kinds(source: &str) -> Vec<TokenKind> {
    lex(source)
        .expect("source should lex")
        .into_iter()
        .map(|token| token.kind)
        .collect()
}

#[test]
fn lexes_unicode_escaped_identifier_four_digit_form() {
    // `abc` decodes to `abc`.
    assert_eq!(
        kinds("\\u0061bc"),
        vec![TokenKind::Identifier("abc".to_owned()), TokenKind::Eof]
    );
}

#[test]
fn lexes_unicode_escaped_identifier_braced_form() {
    // `\u{61}` decodes to `a`; an escape may also appear mid-identifier.
    assert_eq!(
        kinds("\\u{61} a\\u{62}c"),
        vec![
            TokenKind::Identifier("a".to_owned()),
            TokenKind::Identifier("abc".to_owned()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn escaped_identifier_carries_had_escape_flag_and_span() {
    let tokens = lex("a\\u0062c = 1").expect("source should lex");
    let ident = &tokens[0];
    assert_eq!(ident.kind, TokenKind::Identifier("abc".to_owned()));
    assert!(ident.had_escape, "escaped identifier should set had_escape");
    // Span covers the raw escaped source `abc` (8 bytes).
    assert_eq!(ident.span, Span::new(0, 8));
}

#[test]
fn plain_identifier_has_no_escape_flag() {
    let tokens = lex("abc").expect("source should lex");
    assert!(!tokens[0].had_escape);
}

#[test]
fn escaped_spelling_of_reserved_word_is_an_identifier_token() {
    // Escaped reserved words remain IdentifierName tokens so property-name
    // grammar can accept them; parser identifier contexts reject them.
    let tokens = lex("\\u{62}reak \\u0069f cl\\u0061ss").expect("source should lex");
    assert_eq!(
        tokens.iter().map(|token| &token.kind).collect::<Vec<_>>(),
        vec![
            &TokenKind::Identifier("break".to_owned()),
            &TokenKind::Identifier("if".to_owned()),
            &TokenKind::Identifier("class".to_owned()),
            &TokenKind::Eof,
        ]
    );
    assert!(tokens[0].had_escape);
    assert!(tokens[1].had_escape);
    assert!(tokens[2].had_escape);
}

#[test]
fn unescaped_keyword_still_lexes_as_keyword() {
    assert_eq!(kinds("break"), vec![TokenKind::Break, TokenKind::Eof]);
    assert_eq!(kinds("class"), vec![TokenKind::Class, TokenKind::Eof]);
}

#[test]
fn var_declaration_with_escaped_value_lexes() {
    // `var` is a keyword (unescaped); `a` is a plain identifier here.
    assert_eq!(
        kinds("var a = 1"),
        vec![
            TokenKind::Var,
            TokenKind::Identifier("a".to_owned()),
            TokenKind::Equal,
            TokenKind::Number("1".to_owned()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn rejects_escape_decoding_to_invalid_identifier_character() {
    // ` ` is a space: not a valid IdentifierStart.
    assert!(lex("\\u0020").is_err());
    // A digit is not a valid IdentifierStart even via escape.
    assert!(lex("\\u0031").is_err()); // `1`
    // Lone surrogates cannot name identifier characters.
    assert!(lex("\\u{d800}").is_err());
}

#[test]
fn rejects_malformed_identifier_unicode_escape() {
    assert!(lex("\\u").is_err());
    assert!(lex("\\u12").is_err());
    assert!(lex("\\u{}").is_err());
    assert!(lex("\\u{61").is_err());
}

#[test]
fn escaped_contextual_keyword_is_plain_identifier_with_flag() {
    // `let`, `yield`, `async` are contextual; the lexer keeps the escaped
    // spelling as an Identifier carrying the flag so the parser can refuse the
    // contextual role.
    for (source, value) in [
        ("\\u{6c}et", "let"),
        ("yi\\u0065ld", "yield"),
        ("async", "async"),
    ] {
        let tokens = lex(source).expect("contextual escaped identifier should lex");
        assert_eq!(tokens[0].kind, TokenKind::Identifier(value.to_owned()));
    }
    assert!(
        lex("\\u{6c}et")
            .expect("lexes")
            .iter()
            .any(|Token { had_escape, .. }| *had_escape)
    );
}

#[test]
fn accepts_non_ascii_identifier_characters() {
    // `café` and a leading non-ASCII letter should lex as identifiers.
    assert_eq!(
        kinds("café"),
        vec![TokenKind::Identifier("café".to_owned()), TokenKind::Eof]
    );
    assert_eq!(
        kinds("\u{00e9}t\u{00e9}"),
        vec![
            TokenKind::Identifier("\u{00e9}t\u{00e9}".to_owned()),
            TokenKind::Eof,
        ]
    );
}

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
    let tokens =
        lex("0x10 0Xf 0b101 0B11 0o77 0O10 0x1_0 0b10_1 0o7_7").expect("source should lex");
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
            TokenKind::Number("0x1_0".to_owned()),
            TokenKind::Number("0b10_1".to_owned()),
            TokenKind::Number("0o7_7".to_owned()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_bigint_literals() {
    let tokens = lex("0n 12n 0xfn 0b101n 0o77n 1_000n").expect("source should lex");
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::BigInt("0".to_owned()),
            TokenKind::BigInt("12".to_owned()),
            TokenKind::BigInt("0xf".to_owned()),
            TokenKind::BigInt("0b101".to_owned()),
            TokenKind::BigInt("0o77".to_owned()),
            TokenKind::BigInt("1_000".to_owned()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn rejects_invalid_bigint_literals() {
    assert!(lex("01n").is_err());
    assert!(lex("1__0n").is_err());
    assert!(lex("1_n").is_err());
    assert!(lex("1.0n").is_err());
}

#[test]
fn lexes_division_after_bigint_literals() {
    let tokens = lex("7n / 2n").expect("source should lex");
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::BigInt("7".to_owned()),
            TokenKind::Slash,
            TokenKind::BigInt("2".to_owned()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_division_after_private_names() {
    let tokens = lex("this.#x / 2").expect("source should lex");
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::This,
            TokenKind::Dot,
            TokenKind::PrivateName("x".to_owned()),
            TokenKind::Slash,
            TokenKind::Number("2".to_owned()),
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
    assert!(lex("0x_1").is_err());
    assert!(lex("0b1_").is_err());
    assert!(lex("0o1__7").is_err());
}

#[test]
fn lexes_decimal_exponent_numeric_literals() {
    let tokens = lex("1e3 1E+3 1e-3 1.25e2 .5e1 1. 1_000 1_2.3_4 1e1_0 .1_5e1_0")
        .expect("source should lex");
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
            TokenKind::Number("1_000".to_owned()),
            TokenKind::Number("1_2.3_4".to_owned()),
            TokenKind::Number("1e1_0".to_owned()),
            TokenKind::Number(".1_5e1_0".to_owned()),
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
    assert!(lex("1__0").is_err());
    assert!(lex("1_").is_err());
    assert!(lex("0_1").is_err());
    assert!(lex("00_1").is_err());
    assert!(lex("08_1").is_err());
    assert!(lex("1_.0").is_err());
    assert!(lex("1._0").is_err());
    assert!(lex("1e_1").is_err());
    assert!(lex("1e1_").is_err());
    assert!(lex("1\\u005F0").is_err());
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
fn skips_source_start_hashbang_comments() {
    for terminator in ["\n", "\r", "\u{2028}", "\u{2029}"] {
        let source = format!("#! /usr/bin/env qjs{terminator}answer");
        assert_eq!(
            kinds(&source),
            vec![TokenKind::Identifier("answer".to_owned()), TokenKind::Eof]
        );
    }
    assert_eq!(kinds("#! no terminator"), vec![TokenKind::Eof]);
}

#[test]
fn hashbang_comment_only_applies_at_source_start() {
    let error = lex("\n#! no longer source start")
        .expect_err("hashbang after a line terminator should stay invalid");
    assert_eq!(
        error.message,
        "`#` must be followed by a private name identifier"
    );
}

#[test]
fn hashbang_comments_can_be_disabled_for_function_body_source() {
    let error = lex_with_options("#! disabled", LexOptions { hashbang: false })
        .expect_err("function body source should not accept hashbang comments");
    assert_eq!(
        error.message,
        "`#` must be followed by a private name identifier"
    );
}

#[test]
fn skips_annex_b_html_like_comments() {
    let tokens = lex("one <!-- ignore\ntwo\n--> skip\nthree").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Identifier("one".to_owned()),
            TokenKind::Identifier("two".to_owned()),
            TokenKind::Identifier("three".to_owned()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn html_close_comment_requires_preceding_line_terminator() {
    let tokens = lex("one-->two").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Identifier("one".to_owned()),
            TokenKind::MinusMinus,
            TokenKind::Greater,
            TokenKind::Identifier("two".to_owned()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn html_close_comment_allows_initial_whitespace() {
    let tokens = lex("   --> skip\none").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![TokenKind::Identifier("one".to_owned()), TokenKind::Eof,]
    );
}

#[test]
fn html_close_comment_allows_block_comment_prefixes() {
    let tokens = lex("/* first */ /* second */--> skip\none").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![TokenKind::Identifier("one".to_owned()), TokenKind::Eof,]
    );
}

#[test]
fn html_close_comment_allows_multiline_block_comment_prefix_after_token() {
    let tokens = lex("0/*\n*/--> skip\none").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Number("0".to_owned()),
            TokenKind::Identifier("one".to_owned()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn html_close_comment_allows_unicode_line_separators() {
    let tokens = lex("counter\u{2028}--> skip\nnext").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Identifier("counter".to_owned()),
            TokenKind::Identifier("next".to_owned()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn skips_ecmascript_whitespace_and_line_terminators() {
    let tokens =
        lex("one\u{0009}\u{000B}\u{000C}\u{0020}\u{00A0}\u{000A}\u{000D}\u{2028}\u{2029}two")
            .expect("source should lex");
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
            TokenKind::TemplateNoSubstitution(template_segment("hello", "hello")),
            TokenKind::TemplateNoSubstitution(template_segment("", "")),
            TokenKind::TemplateNoSubstitution(template_segment("price $5", "price $5")),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_template_literals_with_substitutions() {
    let tokens = lex("`hello ${name}${1 + 2} end`").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::TemplateHead(template_segment("hello ", "hello ")),
            TokenKind::Identifier("name".to_owned()),
            TokenKind::TemplateMiddle(template_segment("", "")),
            TokenKind::Number("1".to_owned()),
            TokenKind::Plus,
            TokenKind::Number("2".to_owned()),
            TokenKind::TemplateTail(template_segment(" end", " end")),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_template_substitution_with_nested_braces() {
    let tokens = lex("`${{ value: 1 }.value}`").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::TemplateHead(template_segment("", "")),
            TokenKind::LeftBrace,
            TokenKind::Identifier("value".to_owned()),
            TokenKind::Colon,
            TokenKind::Number("1".to_owned()),
            TokenKind::RightBrace,
            TokenKind::Dot,
            TokenKind::Identifier("value".to_owned()),
            TokenKind::TemplateTail(template_segment("", "")),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_template_raw_segments() {
    let tokens = lex(r"`\n${1}\t`").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::TemplateHead(template_segment("\n", r"\n")),
            TokenKind::Number("1".to_owned()),
            TokenKind::TemplateTail(template_segment("\t", r"\t")),
            TokenKind::Eof,
        ]
    );
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
fn lexes_legacy_octal_escape_sequences() {
    assert_eq!(
        lex("'\\07\\141'").unwrap()[0].kind,
        TokenKind::String("\u{0007}a".to_owned())
    );
    assert_eq!(
        lex("`\\07\\141`").unwrap()[0].kind,
        TokenKind::TemplateNoSubstitution(template_segment("\u{0007}a", r"\07\141"))
    );
}

#[test]
fn lexes_surrogate_unicode_escapes() {
    let tokens = lex(r#"'\uD800\uDC00'"#).expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    let TokenKind::String(value) = &kinds[0] else {
        panic!("expected string token");
    };
    assert_eq!(value.chars().count(), 2);
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
        vec![
            TokenKind::TemplateNoSubstitution(template_segment("\nABC`", r"\n\x41\u0042\u{43}\`")),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn normalizes_template_line_terminator_sequences() {
    let tokens = lex("`\r\n\n\r\u{2028}\u{2029}`").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::TemplateNoSubstitution(template_segment(
                "\n\n\n\u{2028}\u{2029}",
                "\n\n\n\u{2028}\u{2029}",
            )),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn preserves_template_line_continuations_in_raw_segments() {
    let tokens = lex("`\\\r\n\\\n\\\r\\\u{2028}\\\u{2029}`").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::TemplateNoSubstitution(template_segment(
                "",
                "\\\n\\\n\\\n\\\u{2028}\\\u{2029}",
            )),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_private_names() {
    let tokens = lex("#x #_foo #$bar1").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::PrivateName("x".to_owned()),
            TokenKind::PrivateName("_foo".to_owned()),
            TokenKind::PrivateName("$bar1".to_owned()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_escaped_private_names() {
    let tokens = lex(r"#\u{6F} #\u2118 #ZW_\u200C_NJ #ZW_\u200D_J").expect("source should lex");
    let kinds: Vec<_> = tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::PrivateName("o".to_owned()),
            TokenKind::PrivateName("\u{2118}".to_owned()),
            TokenKind::PrivateName("ZW_\u{200C}_NJ".to_owned()),
            TokenKind::PrivateName("ZW_\u{200D}_J".to_owned()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn escaped_private_name_carries_had_escape_flag_and_span() {
    let tokens = lex(r"a.#\u{6F}").expect("source should lex");
    assert_eq!(tokens[2].kind, TokenKind::PrivateName("o".to_owned()));
    assert_eq!(tokens[2].span, Span::new(2, 9));
    assert!(tokens[2].had_escape);
}

#[test]
fn private_name_carries_span() {
    let tokens = lex("a.#x").expect("source should lex");
    assert_eq!(tokens[2].kind, TokenKind::PrivateName("x".to_owned()));
    assert_eq!(tokens[2].span, Span::new(2, 4));
}

#[test]
fn rejects_bare_hash() {
    let error = lex("#").expect_err("bare `#` should fail");
    assert_eq!(
        error.message,
        "`#` must be followed by a private name identifier"
    );
    let error = lex("# x").expect_err("`#` followed by space should fail");
    assert_eq!(
        error.message,
        "`#` must be followed by a private name identifier"
    );
}

fn template_segment(cooked: &str, raw: &str) -> TemplateSegment {
    TemplateSegment {
        cooked: cooked.to_owned(),
        raw: raw.to_owned(),
    }
}

#[test]
fn lexes_declaration_keywords() {
    let tokens =
            lex(
                "this var let const if else while do for switch case default try catch finally break continue function class extends return throw debugger typeof void in delete new instanceof variable",
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
            TokenKind::Class,
            TokenKind::Extends,
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
    let tokens = lex("{}[](),.:?%!<>|&^~\\=").expect("source should lex");
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
            (TokenKind::Backslash, Span::new(18, 19)),
            (TokenKind::Equal, Span::new(19, 20)),
            (TokenKind::Eof, Span::new(20, 20)),
        ]
    );
}

#[test]
fn lexes_regexp_escaped_parens_with_spans() {
    let tokens = lex(r#"/\(\)/;"#).expect("source should lex");
    let actual: Vec<_> = tokens
        .into_iter()
        .map(|token| (token.kind, token.span))
        .collect();
    assert_eq!(
        actual,
        vec![
            (
                TokenKind::RegularExpression {
                    pattern: r#"\(\)"#.to_owned(),
                    flags: String::new(),
                },
                Span::new(0, 6)
            ),
            (TokenKind::Semicolon, Span::new(6, 7)),
            (TokenKind::Eof, Span::new(7, 7)),
        ]
    );
}

#[test]
fn lexes_regexp_literal_with_escaped_slash_and_braced_unicode_escape() {
    let tokens = lex(r#"/\// + /\u{1d306}/u"#).expect("source should lex");
    let actual: Vec<_> = tokens
        .into_iter()
        .map(|token| (token.kind, token.span))
        .collect();
    assert_eq!(
        actual,
        vec![
            (
                TokenKind::RegularExpression {
                    pattern: r#"\/"#.to_owned(),
                    flags: String::new(),
                },
                Span::new(0, 4)
            ),
            (TokenKind::Plus, Span::new(5, 6)),
            (
                TokenKind::RegularExpression {
                    pattern: r#"\u{1d306}"#.to_owned(),
                    flags: "u".to_owned(),
                },
                Span::new(7, 19)
            ),
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
