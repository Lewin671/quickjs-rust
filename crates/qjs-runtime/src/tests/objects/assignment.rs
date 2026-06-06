use crate::{Value, eval};

#[test]
fn evaluates_member_assignment() {
    assert_eq!(
        eval("let o = {}; o.answer = 42; o.answer;"),
        Ok(Value::Number(42.0))
    );
    assert_eq!(
        eval("let key = 'answer'; let o = {}; o[key] = 7; o.answer;"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("let seen = 0; let o = { set answer(value) { seen = value; } }; o.answer = 9; seen;"),
        Ok(Value::Number(9.0))
    );
    assert_eq!(
        eval("this.answer = 42; this.answer;"),
        Ok(Value::Number(42.0))
    );
    assert_eq!(eval("this === this;"), Ok(Value::Boolean(true)));
}
