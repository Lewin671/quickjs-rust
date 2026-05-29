use crate::{Value, eval};

#[test]
fn evaluates_string_objects() {
    assert_eq!(
        eval("typeof new String('abc');"),
        Ok(Value::String("object".to_owned()))
    );
    assert_eq!(
        eval("let s = new String('abc'); s.constructor === String;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let s = new String('abc'); s.valueOf();"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval("let s = new String('abc'); s.toString();"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(eval("new String('abc').length;"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("let s = new String('abc'); s[1];"),
        Ok(Value::String("b".to_owned()))
    );
    assert_eq!(
        eval("let s = new String('abc'); try { s.length = 1; } catch (error) {} s.length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("let s = new String('abc'); s == 'abc';"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let s = new String('abc'); s !== 'abc';"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new String('abc'));"),
        Ok(Value::String("[object String]".to_owned()))
    );
    assert_eq!(
        eval("new String('abc').charAt(2);"),
        Ok(Value::String("c".to_owned()))
    );
}
