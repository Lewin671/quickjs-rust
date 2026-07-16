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
    assert_eval("typeof 1n;", Value::String("bigint".to_owned().into()));
    // Unary `-`/`~` use ToNumeric, so a non-primitive that becomes a BigInt
    // uses the BigInt operation instead of throwing in ToNumber.
    assert_eval("-Object(1n) === -1n;", Value::Boolean(true));
    assert_eval("~Object(1n) === -2n;", Value::Boolean(true));
    assert_eval(
        "-{ [Symbol.toPrimitive]() { return 3n; } } === -3n;",
        Value::Boolean(true),
    );
    assert_eval("String(1_000n);", Value::String("1000".to_owned().into()));
    assert_eval("BigInt('0x10') === 16n;", Value::Boolean(true));
    assert_eval("BigInt(44) === 44n;", Value::Boolean(true));
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
    assert_eval(
        "function isConstructor(f) { try { Reflect.construct(function(){}, [], f); } catch (e) { return false; } return true; } isConstructor(BigInt);",
        Value::Boolean(true),
    );
}

#[test]
fn evaluates_bigint_arithmetic_and_equality() {
    assert_eval("String(1n + 2n);", Value::String("3".to_owned().into()));
    assert_eval("String(7n - 4n);", Value::String("3".to_owned().into()));
    assert_eval("String(3n * 4n);", Value::String("12".to_owned().into()));
    assert_eval("String(7n / 2n);", Value::String("3".to_owned().into()));
    assert_eval("String(7n % 2n);", Value::String("1".to_owned().into()));
    assert_eval("String(-5n);", Value::String("-5".to_owned().into()));
    assert_eval("String(~0n);", Value::String("-1".to_owned().into()));
    assert_eval("String(2n ** 8n);", Value::String("256".to_owned().into()));
    assert_eval(
        "let i = 10n; i++; String(i);",
        Value::String("11".to_owned().into()),
    );
    assert_eval("let i = 10n; ++i;", Value::bigint(11.into()));
    assert_eval("let i = 10n; i++;", Value::bigint(10.into()));
    assert_eval(
        "let o = { value: 10n }; o.value--; String(o.value);",
        Value::String("9".to_owned().into()),
    );
    assert_eval("String(5n << 3n);", Value::String("40".to_owned().into()));
    assert_eval("String(5n << -1n);", Value::String("2".to_owned().into()));
    assert_eval("String(5n >> -2n);", Value::String("20".to_owned().into()));
    assert_eval("String(-5n >> 1n);", Value::String("-3".to_owned().into()));
    assert_eval("String(-5n >> 2n);", Value::String("-2".to_owned().into()));
    assert_eval("1n === 1n;", Value::Boolean(true));
    assert_eval("1n == 1;", Value::Boolean(true));
    assert_eval("Object(2n) * 3n;", Value::bigint(6.into()));
    assert_eval("1n === 1;", Value::Boolean(false));
    assert_type_error("1n + 1;");
    assert_type_error("1n >> 1;");
    assert_type_error("1n >>> 0n;");
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
    assert_type_error("BigInt.asIntN(0, 0);");
    assert_type_error("BigInt.asIntN(0, Object(0));");
    assert_type_error("BigInt.asIntN(0, { valueOf: function() { return 0; } });");
    assert_type_error("BigInt.asUintN(0, 0);");
    assert_type_error("BigInt.asUintN(0, Object(0));");
    assert_type_error("BigInt.asUintN(0, { valueOf: function() { return 0; } });");
    assert_eval("(10n).toString(16);", Value::String("a".to_owned().into()));
    assert_eval("Object(1n).valueOf() === 1n;", Value::Boolean(true));
    assert_eval(
        "Object.prototype.toString.call(1n);",
        Value::String("[object BigInt]".to_owned().into()),
    );
    assert_eval(
        "Object.prototype.toString.call(Object(1n));",
        Value::String("[object BigInt]".to_owned().into()),
    );
    assert_eval("Object(1n) == 1n;", Value::Boolean(true));
    assert_eval(
        "let gets = 0; let BigIntToString = BigInt.prototype.toString; Object.defineProperty(BigInt.prototype, 'toString', { get: function() { gets = gets + 1; return BigIntToString; }, configurable: true }); ({ '1': 1 })[Object(1n)]; gets;",
        Value::Number(1.0),
    );
    assert_eval(
        "let BigIntValueOf = BigInt.prototype.valueOf; Object.defineProperty(BigInt.prototype, 'toString', { value: undefined, configurable: true }); Object.defineProperty(BigInt.prototype, 'valueOf', { get: function() { return function() { return BigIntValueOf.call(this) * 2n; }; }, configurable: true }); ''.concat(Object(1n));",
        Value::String("2".to_owned().into()),
    );
    assert_eval(
        "let d = Object.getOwnPropertyDescriptor(BigInt.prototype, Symbol.toStringTag); d.value + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;",
        Value::String("BigInt:false:false:true".to_owned().into()),
    );
    assert_eval("Number(10n);", Value::Number(10.0));
    assert_type_error("BigInt.prototype.valueOf.call(1);");
    assert_type_error("JSON.stringify(1n);");
}

#[test]
fn bigint_string_relational_uses_exact_comparison() {
    // A String operand of a BigInt relational comparison is parsed via
    // StringToBigInt (exact), not coerced to a lossy f64.
    assert_eq!(
        eval("9007199254740993n > '9007199254740992';"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("'0x10' > 15n;"), Ok(Value::Boolean(true)));
    // A non-integer or non-numeric string is undefined -> every comparison false.
    assert_eq!(eval("2n > '1.5';"), Ok(Value::Boolean(false)));
    assert_eq!(eval("2n < '1.5';"), Ok(Value::Boolean(false)));
    assert_eq!(eval("1n > 'abc';"), Ok(Value::Boolean(false)));
    // A StringNumericLiteral allows at most one leading sign; a doubled sign is
    // not a valid integer, so StringToBigInt is undefined and the comparison
    // is false (a single sign still parses).
    assert_eq!(eval("1n > '++0';"), Ok(Value::Boolean(false)));
    assert_eq!(eval("1n > '--0';"), Ok(Value::Boolean(false)));
    assert_eq!(eval("'++1' > 0n;"), Ok(Value::Boolean(false)));
    assert_eq!(eval("'-1' < 0n;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'+2' > 1n;"), Ok(Value::Boolean(true)));
}
