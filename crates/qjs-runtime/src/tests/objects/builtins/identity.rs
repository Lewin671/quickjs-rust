use crate::{Value, eval};

#[test]
fn evaluates_object_identity_builtins() {
    assert_eq!(eval("Object.is.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Object.is(NaN, NaN);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Object.is(+0, -0);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Object.is(-0, -0);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Object.is(1, 1);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Object.is(1, '1');"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("let object = {}; Object.is(object, object);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Object.is({}, {});"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Object.is();"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Object.is(0);"), Ok(Value::Boolean(false)));
}
