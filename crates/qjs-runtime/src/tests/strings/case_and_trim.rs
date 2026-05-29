use crate::{Value, eval};

#[test]
fn evaluates_string_case_and_trim_builtins() {
    assert_eq!(
        eval("'abc'.toString();"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval("'abc'.valueOf();"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval("String.prototype.toLowerCase.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("String.prototype.toUpperCase.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("'AbC123'.toLowerCase();"),
        Ok(Value::String("abc123".to_owned()))
    );
    assert_eq!(
        eval("'AbC123'.toUpperCase();"),
        Ok(Value::String("ABC123".to_owned()))
    );
    assert_eq!(
        eval("'  abc  '.trim();"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval("'  abc  '.trimStart();"),
        Ok(Value::String("abc  ".to_owned()))
    );
    assert_eq!(
        eval("'  abc  '.trimLeft();"),
        Ok(Value::String("abc  ".to_owned()))
    );
    assert_eq!(
        eval("'  abc  '.trimEnd();"),
        Ok(Value::String("  abc".to_owned()))
    );
    assert_eq!(
        eval("'  abc  '.trimRight();"),
        Ok(Value::String("  abc".to_owned()))
    );
    assert_eq!(
        eval("String.prototype.trim.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("String.prototype.trimStart.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("String.prototype.trimEnd.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("String.prototype.trimLeft.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("String.prototype.trimRight.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("String.prototype.trimLeft === String.prototype.trimStart;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("String.prototype.trimRight === String.prototype.trimEnd;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(String.prototype, 'charAt').enumerable;"),
        Ok(Value::Boolean(false))
    );
}
