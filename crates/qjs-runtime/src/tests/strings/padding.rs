use crate::{Value, eval};

#[test]
fn evaluates_string_padding_and_repeat_builtins() {
    assert_eq!(
        eval("String.prototype.padStart.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("String.prototype.padEnd.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("'abc'.padStart(7, 'def');"),
        Ok(Value::String("defdabc".to_owned()))
    );
    assert_eq!(
        eval("'abc'.padEnd(7, 'def');"),
        Ok(Value::String("abcdefd".to_owned()))
    );
    assert_eq!(
        eval("'abc'.padStart(5);"),
        Ok(Value::String("  abc".to_owned()))
    );
    assert_eq!(
        eval("'abc'.padEnd(5);"),
        Ok(Value::String("abc  ".to_owned()))
    );
    assert_eq!(
        eval("'abc'.padStart(5, '');"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval("'abc'.padEnd(2, '*');"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval("'ab'.repeat(3);"),
        Ok(Value::String("ababab".to_owned()))
    );
    assert_eq!(eval("'ab'.repeat(0);"), Ok(Value::String(String::new())));
    assert_eq!(
        eval("'ab'.repeat(2.8);"),
        Ok(Value::String("abab".to_owned()))
    );
    assert!(eval("'ab'.repeat(-1);").is_err());
    assert!(eval("'ab'.repeat(Infinity);").is_err());
}
