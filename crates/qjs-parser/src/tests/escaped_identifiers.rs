use qjs_ast::{Expr, Stmt, VarKind};

use crate::parse_script;

#[test]
fn escaped_identifier_value_is_decoded() {
    // `\u{61}bc` names the binding `abc`.
    let script = parse_script("var \\u{61}bc = 1;").expect("escaped identifier should parse");
    let [Stmt::VarDecl { declarations, .. }] = script.body.as_slice() else {
        panic!("expected a var declaration");
    };
    assert_eq!(declarations[0].binding.names(), vec!["abc"]);
}

#[test]
fn escaped_let_is_not_a_let_declaration() {
    // `\u{6c}et x` is two identifiers (`let`, `x`), not a `let` declaration.
    // It parses as an expression statement referencing `let`, followed by `x`,
    // which is a syntax error (no separator) -- the key property is that it is
    // not parsed as a LexicalDeclaration introducing `x`.
    let script = parse_script("\\u{6c}et;").expect("escaped let identifier should parse");
    let [Stmt::Expr(Expr::Identifier { name, .. })] = script.body.as_slice() else {
        panic!("expected an expression statement referencing `let`");
    };
    assert_eq!(name, "let");
}

#[test]
fn escaped_async_is_a_valid_for_of_lhs() {
    // The `for (async of x)` restriction is a lookahead on the literal `async`
    // token; an escaped `async` is a plain identifier LHS and is valid.
    parse_script("for (\\u0061sync of [7]);")
        .expect("escaped async should be a valid for-of left-hand side");
    // The unescaped form remains an early error.
    parse_script("for (async of [7]);").expect_err("unescaped async for-of LHS is an early error");
}

#[test]
fn unescaped_let_is_still_a_declaration() {
    let script = parse_script("let x = 1;").expect("let declaration should parse");
    let [Stmt::VarDecl { kind, .. }] = script.body.as_slice() else {
        panic!("expected a let declaration");
    };
    assert_eq!(*kind, VarKind::Let);
}

#[test]
fn escaped_reserved_word_binding_is_rejected_in_strict_mode() {
    // `package` is a strict-mode future reserved word; an escaped spelling does
    // not change its StringValue, so it is still rejected.
    assert!(parse_script("\"use strict\"; var packag\\u0065 = 1;").is_err());
    assert!(parse_script("\"use strict\"; var package = 1;").is_err());
    assert!(parse_script("\"use strict\"; var yi\\u0065ld = 1;").is_err());
}

#[test]
fn escaped_reserved_word_binding_is_allowed_in_sloppy_mode() {
    parse_script("var packag\\u0065 = 1;").expect("`package` is a valid identifier in sloppy mode");
    parse_script("var package = 1;").expect("`package` is a valid identifier in sloppy mode");
}

#[test]
fn escaped_reserved_word_object_binding_shorthand_is_rejected() {
    // An object binding shorthand `{ x }` is a binding identifier, so a
    // reserved word -- including an escaped spelling whose StringValue is still
    // reserved -- is a SyntaxError in every binding context.
    assert!(parse_script("var { cl\\u0061ss } = obj;").is_err());
    assert!(parse_script("var { break } = obj;").is_err());
    assert!(parse_script("function f({ \\u0074his }) {}").is_err());
    assert!(parse_script("var { ...cl\\u0061ss } = obj;").is_err());
    // Non-reserved shorthand bindings keep parsing.
    parse_script("var { ok, also } = obj;").expect("plain shorthand bindings parse");
    parse_script("var { ...rest } = obj;").expect("plain rest binding parses");
}

#[test]
fn escaped_always_reserved_word_is_rejected_as_identifier() {
    // `\u{62}reak` decodes to `break`, an unconditionally reserved word, which
    // may be an IdentifierName but not a binding or identifier reference.
    assert!(parse_script("var \\u{62}reak = 1;").is_err());
    assert!(parse_script("\\u0069f (true) {}").is_err());
    assert!(parse_script("cl\\u0061ss;").is_err());
}

#[test]
fn escaped_get_set_accessor_keywords_are_rejected() {
    // `get`/`set` cannot spell the accessor contextual keyword, so
    // `get m() {}` becomes a syntax error (a property name with no `:`).
    assert!(parse_script("({ g\\u0065t m() {} });").is_err());
    assert!(parse_script("({ s\\u0065t m(v) {} });").is_err());
}

#[test]
fn escaped_async_method_modifier_is_rejected() {
    // `async m() {}` is not an async method; `async` is treated as a
    // property name, making the following method name a syntax error.
    assert!(parse_script("({ \\u0061sync m() {} });").is_err());
}

#[test]
fn enum_is_rejected_as_identifier() {
    // `enum` is an unconditionally reserved word per ECMA-262.
    assert!(parse_script("enum = 1;").is_err());
    assert!(parse_script("var enum;").is_err());
    assert!(parse_script("var enum = 1;").is_err());
    assert!(parse_script("let enum;").is_err());
}

#[test]
fn escaped_reserved_word_label_is_rejected() {
    // Escaped spellings of reserved words cannot serve as labels because the
    // StringValue still matches the reserved word (ECMA-262 12.1).
    assert!(parse_script("f\\u0061lse: ;").is_err());
    assert!(parse_script("n\\u0075ll: ;").is_err());
    assert!(parse_script("tr\\u0075e: ;").is_err());
    // `enum` is an unconditionally reserved word and may not be a label.
    assert!(parse_script("enum: ;").is_err());
}

#[test]
fn escaped_let_is_rejected_as_binding_in_strict_mode() {
    // The plain `let` keyword has a dedicated token path; the escaped spelling
    // arrives as an identifier whose StringValue is `let`, which is a reserved
    // word in strict-mode code and may not name a binding (ECMA-262 13.1.1).
    assert!(parse_script("\"use strict\"; function l\\u0065t() {}").is_err());
    assert!(parse_script("\"use strict\"; var l\\u0065t = 1;").is_err());
    assert!(parse_script("\"use strict\"; class l\\u0065t {}").is_err());
    // Sloppy-mode `let` (escaped or not) stays a valid binding name.
    assert!(parse_script("function l\\u0065t() { return 1; }").is_ok());
    assert!(parse_script("var l\\u0065t = 1;").is_ok());
}

#[test]
fn unescaped_get_set_async_are_still_keywords_or_names() {
    // The unescaped accessor/method forms keep working.
    assert!(parse_script("({ get m() { return 1; } });").is_ok());
    assert!(parse_script("({ set m(v) {} });").is_ok());
    assert!(parse_script("({ async m() {} });").is_ok());
    // And the bare spellings remain valid property names.
    assert!(parse_script("({ get: 1, set: 2, async: 3 });").is_ok());
}
