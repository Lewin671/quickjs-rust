//! Async functions parse (T007 S2) but evaluation is not yet implemented
//! (S3): every async form compiles to a structured "not yet supported" error.

use crate::eval;

fn expect_async_unsupported(source: &str) {
    let error = eval(source).expect_err("async forms should not evaluate yet");
    assert!(
        error
            .message
            .contains("async functions are not yet supported"),
        "unexpected error for {source:?}: {}",
        error.message
    );
}

#[test]
fn async_function_declaration_is_unsupported() {
    expect_async_unsupported("async function f() {} f();");
}

#[test]
fn async_function_expression_is_unsupported() {
    expect_async_unsupported("(async function () {})();");
}

#[test]
fn async_arrow_is_unsupported() {
    expect_async_unsupported("(async () => 1)();");
}

#[test]
fn await_expression_is_unsupported() {
    expect_async_unsupported("async function f() { await 1; } f();");
}

#[test]
fn async_method_is_unsupported() {
    expect_async_unsupported("({ async m() {} }).m();");
}

#[test]
fn async_generator_is_unsupported() {
    expect_async_unsupported("async function* g() {} g();");
}

#[test]
fn for_await_of_is_unsupported() {
    expect_async_unsupported("async function f() { for await (const x of []) {} } f();");
}
