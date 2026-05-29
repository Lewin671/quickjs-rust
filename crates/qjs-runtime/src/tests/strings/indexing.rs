use crate::{Value, eval};

#[test]
fn evaluates_string_member_access() {
    assert_eq!(eval("'abc'.length;"), Ok(Value::Number(3.0)));
    assert_eq!(eval("''.length;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("'abc'[0];"), Ok(Value::String("a".to_owned())));
    assert_eq!(eval("'abc'['1'];"), Ok(Value::String("b".to_owned())));
    assert_eq!(eval("'abc'[3];"), Ok(Value::Undefined));
    assert_eq!(eval("'abc'['01'];"), Ok(Value::Undefined));
}
