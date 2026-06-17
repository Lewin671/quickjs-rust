//! `using` / `await using` declarations (Explicit Resource Management). Both
//! are contextual: a declaration only when an identifier follows on the same
//! line; otherwise `using`/`await` are ordinary identifiers.

use qjs_ast::{Stmt, VarKind};

use crate::{parse_module, parse_script};

/// Parses `using`/`await using` inside a block (it is a Syntax Error at the
/// script top level) and returns the declaration kind.
fn block_using_kind(decl_source: &str) -> VarKind {
    let script = parse_script(&format!("{{ {decl_source} }}")).expect("source should parse");
    let [Stmt::Block { body, .. }] = script.body.as_slice() else {
        panic!("expected a block, got {:?}", script.body);
    };
    let [Stmt::VarDecl { kind, .. }] = body.as_slice() else {
        panic!("expected a single var declaration, got {body:?}");
    };
    *kind
}

#[test]
fn parses_using_declaration() {
    assert_eq!(block_using_kind("using x = res;"), VarKind::Using);
    assert_eq!(block_using_kind("using a = one, b = two;"), VarKind::Using);
}

#[test]
fn using_is_rejected_at_script_top_level() {
    // Allowed only inside a block, function/generator body, for-head, or class
    // static block -- not directly at the Script (or eval) top level.
    assert!(parse_script("using x = res;").is_err());
    parse_script("{ using x = res; }").expect("using in a block parses");
    parse_script("function f() { using x = res; }").expect("using in a function parses");
}

#[test]
fn parses_await_using_declaration_in_module() {
    let script = parse_module("await using x = res;").expect("module source should parse");
    let [Stmt::VarDecl { kind, .. }] = script.body.as_slice() else {
        panic!("expected a single var declaration");
    };
    assert_eq!(*kind, VarKind::AwaitUsing);
}

#[test]
fn using_requires_an_initializer() {
    assert!(parse_script("{ using x; }").is_err());
}

#[test]
fn using_binds_only_identifiers() {
    // `using` followed by `[`/`{` is never a `using` declaration (there is no
    // binding-pattern form); these are rejected as invalid expressions.
    assert!(parse_script("{ using [] = null; }").is_err());
    assert!(parse_script("{ using {} = null; }").is_err());
}

#[test]
fn using_followed_by_newline_is_an_identifier() {
    // ASI splits `using` (an identifier reference) from the next line.
    let script = parse_script("using\nx;").expect("source should parse");
    assert!(matches!(
        script.body.as_slice(),
        [Stmt::Expr(_), Stmt::Expr(_)]
    ));
}

#[test]
fn using_as_plain_identifier_is_an_expression() {
    let script = parse_script("using + 1;").expect("source should parse");
    assert!(matches!(script.body.as_slice(), [Stmt::Expr(_)]));
}

#[test]
fn await_using_outside_async_is_an_identifier_sequence() {
    // `await` is not a keyword in script code, so `await using` is not a
    // declaration here.
    let script = parse_script("await;").expect("source should parse");
    assert!(matches!(script.body.as_slice(), [Stmt::Expr(_)]));
}

#[test]
fn using_is_rejected_as_a_single_statement_body() {
    assert!(parse_script("if (cond) using x = res;").is_err());
    assert!(parse_script("label: using x = res;").is_err());
}

#[test]
fn parses_using_in_for_heads() {
    // for-of and for-await-of allow `using`; the C-style for head allows it
    // with an initializer; for-in does not.
    parse_script("for (using x of iter) {}").expect("for-of using parses");
    parse_module("for await (using x of iter) {}").expect("for-await-of using parses");
    parse_script("for (using x = res; cond; ) {}").expect("C-style for using parses");
    assert!(parse_script("for (using x in obj) {}").is_err());
}
