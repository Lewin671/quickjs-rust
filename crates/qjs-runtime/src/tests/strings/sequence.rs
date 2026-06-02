use crate::{Value, eval};

#[test]
fn evaluates_string_sequence_builtins() {
    assert_eq!(
        eval("'a'.concat('b', 3, true);"),
        Ok(Value::String("ab3true".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.slice(1, 4);"),
        Ok(Value::String("bcd".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.slice(-3);"),
        Ok(Value::String("def".to_owned()))
    );
    assert_eq!(
        eval(
            "function f() {} f.valueOf = function() { return 'gnulluna'; }; f.toString = function() { return f; }; Function.prototype.slice = String.prototype.slice; f.slice(null, Function().slice(f, 5).length);"
        ),
        Ok(Value::String("gnull".to_owned()))
    );
    assert_eq!(
        eval("String.prototype.split.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("'hello'.split('l').join('|');"),
        Ok(Value::String("he||o".to_owned()))
    );
    assert_eq!(
        eval("'hello'.split('l', 2).join('|');"),
        Ok(Value::String("he|".to_owned()))
    );
    assert_eq!(
        eval("'hello'.split(undefined).join('|');"),
        Ok(Value::String("hello".to_owned()))
    );
    assert_eq!(
        eval("'abc'.split('', 2).join('|');"),
        Ok(Value::String("a|b".to_owned()))
    );
    assert_eq!(eval("'abc'.split('x').length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'abc'.split('b', 0).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("'hello'.split(/l/).join('|');"),
        Ok(Value::String("he||o".to_owned()))
    );
    assert_eq!(
        eval("'hello'.split(/l/, 2).join('|');"),
        Ok(Value::String("he|".to_owned()))
    );
    assert_eq!(
        eval("'abc'.split(/[a-z]/).join('|');"),
        Ok(Value::String("|||".to_owned()))
    );
    assert_eq!(
        eval("'hello'.split(new RegExp).join('|');"),
        Ok(Value::String("h|e|l|l|o".to_owned()))
    );
    assert_eq!(
        eval(
            "let called = false; let separator = { toString: function() { called = true; return 'x'; } }; 'abc'.split(separator, 0); called;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("String.prototype.substring.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("String.prototype.substr.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("'abcdef'.substring(1, 4);"),
        Ok(Value::String("bcd".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substring(4, 1);"),
        Ok(Value::String("bcd".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substring(-3, 2);"),
        Ok(Value::String("ab".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substring(3);"),
        Ok(Value::String("def".to_owned()))
    );
    assert_eq!(
        eval(
            "function f() {} f.valueOf = function() { return 'gnulluna'; }; Function.prototype.substring = String.prototype.substring; f.substring(null, Function());"
        ),
        Ok(Value::String(String::new()))
    );
    assert_eq!(
        eval("'abcdef'.substr(1, 3);"),
        Ok(Value::String("bcd".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substr(-2);"),
        Ok(Value::String("ef".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substr(-20, 2);"),
        Ok(Value::String("ab".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substr(2, -1);"),
        Ok(Value::String(String::new()))
    );
    assert_eq!(
        eval("'abcdef'.substr(2, 2.8);"),
        Ok(Value::String("cd".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substr(Infinity, 1);"),
        Ok(Value::String(String::new()))
    );
    assert_eq!(
        eval(
            "let caught = false; try { ''.repeat(Infinity); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { ''.repeat(-1); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}
