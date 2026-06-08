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
    assert_eval("BigInt('') === 0n;", Value::Boolean(true));
    assert_eval("BigInt('   -197   ') === -197n;", Value::Boolean(true));
    assert_eval(
        "BigInt({ valueOf: function() { return 44; }, toString: function() { throw 'unreachable'; } }) === 44n;",
        Value::Boolean(true),
    );
    assert_type_error("new BigInt(1);");
    assert_type_error("BigInt(undefined);");
    assert_type_error("BigInt(null);");
    assert_syntax_error("BigInt('1_0');");
    assert_syntax_error("BigInt('-0x1');");
    assert_syntax_error("BigInt('0x');");
    assert_syntax_error("BigInt('0b');");
    assert_syntax_error("BigInt('0o');");
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
    assert_eval(
        "let i = 10n; i++; String(i);",
        Value::String("11".to_owned()),
    );
    assert_eval("let i = 10n; ++i;", Value::BigInt(11.into()));
    assert_eval("let i = 10n; i++;", Value::BigInt(10.into()));
    assert_eval(
        "let o = { value: 10n }; o.value--; String(o.value);",
        Value::String("9".to_owned()),
    );
    assert_eval("1n === 1n;", Value::Boolean(true));
    assert_eval("1n == 1;", Value::Boolean(true));
    assert_eval("1n === 1;", Value::Boolean(false));
    assert_type_error("1n + 1;");
}

#[test]
fn evaluates_bigint_statics_and_prototype_methods() {
    assert_eval("BigInt.asIntN(2, 3n) === -1n;", Value::Boolean(true));
    assert_eval("BigInt.asUintN(2, -1n) === 3n;", Value::Boolean(true));
    assert_eval("BigInt.asIntN(undefined, 1n) === 0n;", Value::Boolean(true));
    assert_eval("BigInt.asIntN('foo', 1n) === 0n;", Value::Boolean(true));
    assert_eval("BigInt.asIntN(-0.9, 1n) === 0n;", Value::Boolean(true));
    assert_eval("BigInt.asIntN(3.9, 10n) === 2n;", Value::Boolean(true));
    assert_eval(
        "BigInt.asIntN({ valueOf: function() { return 3; } }, '10') === 2n;",
        Value::Boolean(true),
    );
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
    assert_eval("Number(10n);", Value::Number(10.0));
    assert_type_error("BigInt.prototype.valueOf.call(1);");
    assert_type_error("JSON.stringify(1n);");
}
