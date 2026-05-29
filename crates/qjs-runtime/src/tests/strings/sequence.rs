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
        eval("String.prototype.substring.length;"),
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
}
