use crate::{Value, eval};

#[test]
fn evaluates_arithmetic() {
    assert_eq!(eval("1 + 2 * 3;"), Ok(Value::Number(7.0)));
    assert_eq!(eval("0x10 + 0b11 + 0o7;"), Ok(Value::Number(26.0)));
    assert_eq!(eval("0Xf + 0B10 + 0O10;"), Ok(Value::Number(25.0)));
    assert_eq!(eval("1e3 + 1E+2 + 1e-1 + .5e1;"), Ok(Value::Number(1105.1)));
    assert_eq!(eval("true + true;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("true * 2;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("2 ** 3;"), Ok(Value::Number(8.0)));
    assert_eq!(eval("2 ** 3 ** 2;"), Ok(Value::Number(512.0)));
    assert_eq!(eval("3 * 2 ** 3;"), Ok(Value::Number(24.0)));
    assert_eq!(eval("2 ** -1 * 2;"), Ok(Value::Number(1.0)));
}

#[test]
fn evaluates_bitwise_and_shift_expressions() {
    assert_eq!(eval("5 & 3;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("5 | 2;"), Ok(Value::Number(7.0)));
    assert_eq!(eval("5 ^ 3;"), Ok(Value::Number(6.0)));
    assert_eq!(eval("2 << 3;"), Ok(Value::Number(16.0)));
    assert_eq!(eval("-8 >> 1;"), Ok(Value::Number(-4.0)));
    assert_eq!(eval("-1 >>> 0;"), Ok(Value::Number(4_294_967_295.0)));
    assert_eq!(eval("~false;"), Ok(Value::Number(-1.0)));
    assert_eq!(eval("1 + 2 << 3;"), Ok(Value::Number(24.0)));
}

#[test]
fn evaluates_string_addition() {
    assert_eq!(eval("'x' + 1;"), Ok(Value::String("x1".to_owned())));
    assert_eq!(eval("`x` + 1;"), Ok(Value::String("x1".to_owned())));
    assert_eq!(eval("`` + `x`;"), Ok(Value::String("x".to_owned())));
    assert_eq!(
        eval(r#""\x41" + "\u0042" + "\u{43}" + "\A";"#),
        Ok(Value::String("ABCA".to_owned()))
    );
    assert_eq!(eval("\"a\\\nb\";"), Ok(Value::String("ab".to_owned())));
    assert_eq!(eval("1 + 'x';"), Ok(Value::String("1x".to_owned())));
    assert_eq!(eval("'x' + true;"), Ok(Value::String("xtrue".to_owned())));
    assert_eq!(eval("'x' + null;"), Ok(Value::String("xnull".to_owned())));
    assert_eq!(
        eval("'x' + undefined;"),
        Ok(Value::String("xundefined".to_owned()))
    );
}

#[test]
fn evaluates_comparison_and_equality() {
    assert_eq!(eval("1 + 2 * 3 >= 7;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("1 + 1 === 2;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("1 !== 2;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("null == undefined;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("null != undefined;"), Ok(Value::Boolean(false)));
    assert_eq!(eval("'1' == 1;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("1 == '1';"), Ok(Value::Boolean(true)));
    assert_eq!(eval("true == 1;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("false == 0;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("false == '';"), Ok(Value::Boolean(true)));
    assert_eq!(eval("NaN == NaN;"), Ok(Value::Boolean(false)));
    assert_eq!(eval("'x' == 1;"), Ok(Value::Boolean(false)));
    assert_eq!(eval("'1' === 1;"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("function C() {} let instance = new C(); instance instanceof C;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function C() {} function D() {} let instance = new C(); instance instanceof D;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("function C() {} 1 instanceof C;"),
        Ok(Value::Boolean(false))
    );
    assert!(eval("let object = {}; object instanceof {};").is_err());
    assert!(
        eval("function C() {} C.prototype = 1; let object = {}; object instanceof C;").is_err()
    );
}

#[test]
fn evaluates_logical_expressions() {
    assert_eq!(eval("0 || 5;"), Ok(Value::Number(5.0)));
    assert_eq!(eval("1 && 7;"), Ok(Value::Number(7.0)));
}

#[test]
fn evaluates_nullish_coalescing_expressions() {
    assert_eq!(eval("null ?? 42;"), Ok(Value::Number(42.0)));
    assert_eq!(eval("undefined ?? 42;"), Ok(Value::Number(42.0)));
    assert_eq!(eval("0 ?? 42;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("false ?? 42;"), Ok(Value::Boolean(false)));
    assert_eq!(eval("42 ?? missing;"), Ok(Value::Number(42.0)));
    assert_eq!(eval("null ?? 0 ?? 1;"), Ok(Value::Number(0.0)));
}

#[test]
fn evaluates_conditional_expressions() {
    assert_eq!(eval("true ? 1 : 2;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("false ? 1 : 2;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("let x = true ? 'yes' : 'no'; x;"),
        Ok(Value::String("yes".to_owned()))
    );
    assert_eq!(eval("true ? 1 : missing;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("false ? missing : 2;"), Ok(Value::Number(2.0)));
}

#[test]
fn evaluates_sequence_expressions() {
    assert_eq!(eval("1, 2;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("let x = 0; x = 1, x = x + 2, x;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("let x = 0; while ((x = x + 1, x < 3)) { } x;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_assignment_expressions() {
    assert_eq!(eval("let x = 2; x = x + 3; x;"), Ok(Value::Number(5.0)));
}

#[test]
fn evaluates_update_and_compound_assignment() {
    assert_eq!(eval("let x = 1; x++; x;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("let x = 1; ++x;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("let x = 1; x++;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("let x = false; x++;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("let x = 3; x--; x;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("let x = 1; x += 2; x;"), Ok(Value::Number(3.0)));
    assert_eq!(eval("let x = -3; x **= 3; x;"), Ok(Value::Number(-27.0)));
    assert_eq!(eval("let x = 2; x <<= 3; x;"), Ok(Value::Number(16.0)));
    assert_eq!(eval("let x = -8; x >>= 1; x;"), Ok(Value::Number(-4.0)));
    assert_eq!(
        eval("let x = -1; x >>>= 0; x;"),
        Ok(Value::Number(4_294_967_295.0))
    );
    assert_eq!(eval("let x = 5; x &= 3; x;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("let x = 5; x ^= 3; x;"), Ok(Value::Number(6.0)));
    assert_eq!(eval("let x = 5; x |= 2; x;"), Ok(Value::Number(7.0)));
    assert_eq!(
        eval("let x = 'a'; x += 1; x;"),
        Ok(Value::String("a1".to_owned()))
    );
    assert_eq!(
        eval("let o = { count: 1 }; o.count++; o.count;"),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn evaluates_logical_assignment() {
    assert_eq!(eval("let x = 0; x &&= missing; x;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("let x = 2; x &&= 7; x;"), Ok(Value::Number(7.0)));
    assert_eq!(eval("let x = 0; x ||= 7; x;"), Ok(Value::Number(7.0)));
    assert_eq!(eval("let x = 2; x ||= missing; x;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("let x = null; x ??= 7; x;"), Ok(Value::Number(7.0)));
    assert_eq!(
        eval("let x = undefined; x ??= 8; x;"),
        Ok(Value::Number(8.0))
    );
    assert_eq!(
        eval("let x = false; x ??= missing; x;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let o = { value: 0 }; o.value ||= 3; o.value;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_unary_expressions() {
    assert_eq!(eval("-1 + 3;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("!0;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("+true;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("void 0;"), Ok(Value::Undefined));
    assert_eq!(eval("let x = 0; void (x = 1); x;"), Ok(Value::Number(1.0)));
}

#[test]
fn evaluates_typeof_expressions() {
    assert_eq!(
        eval("typeof undefined;"),
        Ok(Value::String("undefined".to_owned()))
    );
    assert_eq!(
        eval("typeof neverDeclared;"),
        Ok(Value::String("undefined".to_owned()))
    );
    assert_eq!(
        eval("typeof true;"),
        Ok(Value::String("boolean".to_owned()))
    );
    assert_eq!(eval("typeof 1;"), Ok(Value::String("number".to_owned())));
    assert_eq!(eval("typeof 'x';"), Ok(Value::String("string".to_owned())));
    assert_eq!(eval("typeof null;"), Ok(Value::String("object".to_owned())));
    assert_eq!(eval("typeof {};"), Ok(Value::String("object".to_owned())));
    assert_eq!(eval("typeof this;"), Ok(Value::String("object".to_owned())));
    assert_eq!(
        eval("function f() { return 1; } typeof f;"),
        Ok(Value::String("function".to_owned()))
    );
}

#[test]
fn evaluates_delete_operator() {
    assert_eq!(eval("let o = {}; delete o.x;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("let o = { red: 1 }; delete o.red; o.red;"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("let o = { 2: 2 }; delete o[2]; o['2'];"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("let o = {}; Object.defineProperty(o, 'fixed', { value: 1 }); delete o.fixed;"),
        Ok(Value::Boolean(false))
    );
}

#[test]
fn evaluates_in_operator() {
    assert_eq!(
        eval("'answer' in { answer: 42 };"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("'missing' in { answer: 42 };"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let o = {}; o.present = undefined; 'present' in o;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("'length' in [1, 2];"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'call' in function f() {};"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval(
            "let proto = { marker: 1 }; let array = []; Object.setPrototypeOf(array, proto); 'marker' in array;"
        ),
        Ok(Value::Boolean(true))
    );
    assert!(eval("'a' in 1;").is_err());
}
