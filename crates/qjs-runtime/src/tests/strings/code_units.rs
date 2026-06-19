use crate::{Value, eval};

#[test]
fn evaluates_string_code_unit_builtins() {
    assert_eq!(eval("String.prototype.at.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("'abc'.at(1);"),
        Ok(Value::String("b".to_owned().into()))
    );
    assert_eq!(
        eval("'abc'.at(-1);"),
        Ok(Value::String("c".to_owned().into()))
    );
    assert_eq!(eval("'abc'.at(3);"), Ok(Value::Undefined));
    assert_eq!(eval("'abc'.at(-4);"), Ok(Value::Undefined));
    assert_eq!(
        eval("'abc'.at();"),
        Ok(Value::String("a".to_owned().into()))
    );
    assert_eq!(
        eval("'abc'.at(1.9);"),
        Ok(Value::String("b".to_owned().into()))
    );
    assert_eq!(
        eval("String.prototype.charAt.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("'abc'.charAt(1);"),
        Ok(Value::String("b".to_owned().into()))
    );
    assert_eq!(
        eval("'abc'.charAt(9);"),
        Ok(Value::String(::std::rc::Rc::new(String::new())))
    );
    assert_eq!(
        eval("String.prototype.charCodeAt.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("'abc'.charCodeAt(1);"), Ok(Value::Number(98.0)));
    assert_eq!(
        eval("'abc'.charCodeAt(undefined);"),
        Ok(Value::Number(97.0))
    );
    assert_eq!(
        eval("let x = 'abc'.charCodeAt(9); x !== x;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let x = 'abc'.charCodeAt(-1); x !== x;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("'😀'.charCodeAt(0);"), Ok(Value::Number(55_357.0)));
    assert_eq!(eval("'😀'.charCodeAt(1);"), Ok(Value::Number(56_832.0)));
    assert_eq!(
        eval("'\\uD800\\uDC00'.codePointAt(0);"),
        Ok(Value::Number(65_536.0))
    );
    assert_eq!(
        eval("'\\uD800\\uE000'.codePointAt(0);"),
        Ok(Value::Number(55_296.0))
    );
    assert_eq!(
        eval("'\\uD800'.charCodeAt(0);"),
        Ok(Value::Number(55_296.0))
    );
    assert_eq!(
        eval(
            "let object = new Object(42); object.charAt = String.prototype.charAt; object.charAt(false) + object.charAt(true);"
        ),
        Ok(Value::String("42".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = new Object(42); object.charCodeAt = String.prototype.charCodeAt; object.charCodeAt(0) + object.charCodeAt(1);"
        ),
        Ok(Value::Number(102.0))
    );
    assert_eq!(
        eval(
            "let object = { valueOf: 1, toString: function() { throw 'marker'; }, charAt: String.prototype.charAt }; let caught = false; try { object.charAt(); } catch (error) { caught = error === 'marker'; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("String.prototype.codePointAt.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("'abc'.codePointAt(1);"), Ok(Value::Number(98.0)));
    assert_eq!(eval("'abc'.codePointAt(-1);"), Ok(Value::Undefined));
    assert_eq!(eval("'abc'.codePointAt(3);"), Ok(Value::Undefined));
    assert_eq!(eval("'😀'.codePointAt(0);"), Ok(Value::Number(128_512.0)));
    assert_eq!(eval("'😀'.codePointAt(1);"), Ok(Value::Number(56_832.0)));
    assert_eq!(
        eval("let seen = ''; for (var ch of 'ab') { seen += ch; } seen;"),
        Ok(Value::String("ab".to_owned().into()))
    );
    assert_eq!(
        eval("let seen = ''; for (var ch of 'a\\uD801\\uDC28b') { seen += ch + '|'; } seen;"),
        Ok(Value::String("a|𐐨|b|".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let seen = ''; for (var ch of 'a\\uD801b') { seen += ch.length + ':' + ch.charCodeAt(0) + '|'; } seen;"
        ),
        Ok(Value::String("1:97|1:55297|1:98|".to_owned().into()))
    );
    assert_eq!(
        eval("let iterator = 'x'[Symbol.iterator](); iterator[Symbol.iterator]() === iterator;"),
        Ok(Value::Boolean(true))
    );
    assert!(eval("new String.prototype.charAt();").is_err());
}
