use crate::{Value, eval};

#[test]
fn evaluates_string_constructor_and_statics() {
    assert_eq!(
        eval("typeof String;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("String.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("String();"), Ok(Value::String(String::new())));
    assert_eq!(eval("String(123);"), Ok(Value::String("123".to_owned())));
    assert_eq!(eval("String(null);"), Ok(Value::String("null".to_owned())));
    assert_eq!(
        eval("String.fromCharCode(65, 66, 67);"),
        Ok(Value::String("ABC".to_owned()))
    );
    assert_eq!(
        eval("String.fromCodePoint(65, 128512, 67);"),
        Ok(Value::String("A😀C".to_owned()))
    );
    assert_eq!(
        eval("String.fromCodePoint();"),
        Ok(Value::String(String::new()))
    );
    assert_eq!(eval("String.fromCodePoint.length;"), Ok(Value::Number(1.0)));
    assert!(eval("String.fromCodePoint(-1);").is_err());
    assert!(eval("String.fromCodePoint(1.5);").is_err());
    assert!(eval("String.fromCodePoint(1114112);").is_err());
    assert_eq!(
        eval("String.prototype.constructor === String;"),
        Ok(Value::Boolean(true))
    );
}
