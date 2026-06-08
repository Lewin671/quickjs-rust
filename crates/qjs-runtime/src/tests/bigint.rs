use crate::{RuntimeError, Value, eval};

fn assert_eval(source: &str, expected: Value) {
    assert_eq!(eval(source), Ok(expected));
}

fn assert_type_error(source: &str) {
    assert!(matches!(
        eval(source),
        Err(RuntimeError { message, .. }) if message.starts_with("TypeError")
    ));
}

fn assert_syntax_error(source: &str) {
    assert!(matches!(
        eval(source),
        Err(RuntimeError { message, .. }) if message.starts_with("SyntaxError")
    ));
}

#[test]
fn evaluates_bigint_literals_and_constructor() {
    assert_eval("typeof 1n;", Value::String("bigint".to_owned()));
    assert_eval("String(1_000n);", Value::String("1000".to_owned()));
    assert_eval("BigInt('0x10') === 16n;", Value::Boolean(true));
    assert_eval("BigInt(true) === 1n;", Value::Boolean(true));
    assert_type_error("new BigInt(1);");
    assert_type_error("BigInt(undefined);");
    assert_type_error("BigInt(null);");
    assert_syntax_error("BigInt('1_0');");
}

#[test]
fn evaluates_bigint_arithmetic_and_equality() {
    assert_eval("String(1n + 2n);", Value::String("3".to_owned()));
    assert_eval("String(7n - 4n);", Value::String("3".to_owned()));
    assert_eval("String(3n * 4n);", Value::String("12".to_owned()));
    assert_eval("String(7n / 2n);", Value::String("3".to_owned()));
    assert_eval("String(7n % 2n);", Value::String("1".to_owned()));
    assert_eval("String(-5n);", Value::String("-5".to_owned()));
    assert_eval("String(~0n);", Value::String("-1".to_owned()));
    assert_eval("String(2n ** 8n);", Value::String("256".to_owned()));
    assert_eval("1n === 1n;", Value::Boolean(true));
    assert_eval("1n == 1;", Value::Boolean(true));
    assert_eval("1n === 1;", Value::Boolean(false));
    assert_type_error("1n + 1;");
}

#[test]
fn evaluates_bigint_statics_and_prototype_methods() {
    assert_eval("BigInt.asIntN(2, 3n) === -1n;", Value::Boolean(true));
    assert_eval("BigInt.asUintN(2, -1n) === 3n;", Value::Boolean(true));
    assert_eval("(10n).toString(16);", Value::String("a".to_owned()));
    assert_eval("Object(1n).valueOf() === 1n;", Value::Boolean(true));
    assert_eval(
        "Object.prototype.toString.call(1n);",
        Value::String("[object BigInt]".to_owned()),
    );
    assert_eval(
        "Object.prototype.toString.call(Object(1n));",
        Value::String("[object BigInt]".to_owned()),
    );
    assert_type_error("BigInt.prototype.valueOf.call(1);");
    assert_type_error("JSON.stringify(1n);");
}
