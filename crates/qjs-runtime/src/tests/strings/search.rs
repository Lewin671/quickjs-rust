use crate::{Value, eval};

#[test]
fn evaluates_string_search_builtins() {
    assert_eq!(eval("'abc'.startsWith('ab');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'abc'.startsWith('bc', 1);"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("'abc'.startsWith('bc', 2);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("'abc'.endsWith('bc');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'abc'.endsWith('ab', 2);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'abc'.endsWith('bc', 2);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("'abc'.indexOf('b');"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'abc'.indexOf('b', 2);"), Ok(Value::Number(-1.0)));
    assert_eq!(eval("'abc'.includes('b');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'abc'.includes('b', 2);"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("String.prototype.lastIndexOf.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("'canal'.lastIndexOf('a');"), Ok(Value::Number(3.0)));
    assert_eq!(eval("'canal'.lastIndexOf('a', 2);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'canal'.lastIndexOf('x');"), Ok(Value::Number(-1.0)));
    assert_eq!(eval("'abc'.lastIndexOf('', 1);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'abc'.lastIndexOf('', 99);"), Ok(Value::Number(3.0)));
}
