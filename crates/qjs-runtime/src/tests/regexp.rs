use crate::{Value, eval};

#[test]
fn evaluates_regexp_constructor_identity() {
    assert_eq!(
        eval("typeof RegExp;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("RegExp.length;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("new RegExp() instanceof RegExp;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new RegExp());"),
        Ok(Value::String("[object RegExp]".to_owned()))
    );
}
